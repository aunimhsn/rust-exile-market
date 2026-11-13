//! The item block: the name of the copied item in its rarity color, the Check on trade pill,
//! its base type and class (the base and the tier of a waystone), then its facts (the main
//! values of its base, the skills it grants, its requirements, its tags), and the pill of its
//! stats. A click on a state tag
//! (Corrupted, Fractured…) lights it: the offers of that state only.

use std::borrow::Cow;

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Color32, CornerRadius, CursorIcon, FontId, Frame, Label, Layout, Response, RichText,
    Role, Sense, Ui, Vec2, WidgetInfo, vec2,
};

use super::item_stats::{self, LineFacts, Stats};
use super::theme::{color, size, text};
use super::widgets;
use crate::filters::Filters;
use crate::i18n::{Message, NumberTag, Requirement, Translator};
use crate::item::{
    self, DamageKind, Item, ItemState, MAX_GEM_SOCKETS, Rarity, Requirements, TOP_GEM_LEVEL,
    TOP_GEM_QUALITY, WaystoneProperty,
};

/// The space above and under the block.
const PADDING_Y: f32 = 12.0;
/// The space between the name and the Check on trade pill.
const NAME_GAP: f32 = 12.0;
/// The space between the name and the base type.
const BASE_GAP: f32 = 2.0;
/// The space above the facts: the block reads in two groups, what the item is, then its
/// facts.
const FACTS_GAP: f32 = 12.0;
/// The space between two facts.
const FACT_GAP: f32 = 6.0;
/// The space between the base values and the requirements, which look farther apart than
/// the other facts.
const VALUES_REQUIREMENTS_GAP: f32 = 3.0;
/// The space above the pill of the stats: a block with a background carries no air of its
/// own, unlike a line of text.
const STATS_PILL_GAP: f32 = 20.0;
/// The space above the stats.
const STATS_GAP: f32 = 8.0;
/// The space between the pill of the stats and the number of active filters.
const FILTERS_GAP: f32 = 10.0;
/// The space between the base values, across and down when they wrap.
const VALUE_SPACING: Vec2 = vec2(14.0, 4.0);
/// The space between the label of a base value and its number.
const VALUE_NUMBER_GAP: f32 = 5.0;
/// The space between the tags, across and down when they wrap.
const TAG_SPACING: Vec2 = vec2(6.0, 5.0);
/// The space around the text of a tag.
const TAG_PADDING: Vec2 = vec2(7.0, 1.0);
/// The separator of the base type and the class, of the skills and of the requirements; of
/// the base, the item level and the states in the tooltip of an offer.
pub(super) const SEPARATOR: &str = " · ";
/// The largest DPS shown: far over any weapon, under the largest `u32`.
const MAX_DPS: f64 = 1e6;

/// What the player did in the item block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemBlockAction {
    CheckOnTrade,
    /// The pill of the stats: unfolds them, or folds them.
    ToggleStats,
}

/// What a tag is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagKind {
    Plain,
    /// A number of a gem at its top, in the color of its level there.
    Top(Color32),
    /// A state of the item, a filter of the search: gray, the offers with or without that
    /// state; lit by a click, the offers of that state only.
    State(ItemState, StateTag),
}

/// A state tag: its tooltips, and its colors once lit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StateTag {
    /// Gray: the offers with or without that state.
    or_not: Message,
    /// Lit: the offers of that state only.
    only: Message,
    /// Lit: its text, its background, and its background under the pointer.
    text: Color32,
    fill: Color32,
    hovered: Color32,
}

/// The state tags, in the order of the specifications.
const STATE_TAGS: [(ItemState, Message); 6] = [
    (ItemState::Corrupted, Message::TagCorrupted),
    (ItemState::TwiceCorrupted, Message::TagTwiceCorrupted),
    (ItemState::Sanctified, Message::TagSanctified),
    (ItemState::Fractured, Message::TagFractured),
    (ItemState::Mirrored, Message::TagMirrored),
    (ItemState::Unidentified, Message::TagUnidentified),
];

/// The tag of `state`, when it is a filter of the search: every state but Unidentified.
fn state_tag(state: ItemState) -> Option<StateTag> {
    let corrupted = |only| StateTag {
        or_not: Message::StateCorruptedOrNot,
        only,
        text: color::CORRUPTED_STATE,
        fill: color::CORRUPTED_STATE_BG,
        hovered: color::CORRUPTED_STATE_HOVER,
    };
    match state {
        ItemState::Corrupted => Some(corrupted(Message::StateCorruptedOnly)),
        ItemState::TwiceCorrupted => Some(corrupted(Message::StateTwiceCorruptedOnly)),
        ItemState::Sanctified => Some(StateTag {
            or_not: Message::StateSanctifiedOrNot,
            only: Message::StateSanctifiedOnly,
            text: color::SANCTIFIED_STATE,
            fill: color::SANCTIFIED_STATE_BG,
            hovered: color::SANCTIFIED_STATE_HOVER,
        }),
        ItemState::Fractured => Some(StateTag {
            or_not: Message::StateFracturedOrNot,
            only: Message::StateFracturedOnly,
            text: color::FRACTURED_STATE,
            fill: color::FRACTURED_STATE_BG,
            hovered: color::FRACTURED_STATE_HOVER,
        }),
        ItemState::Mirrored => Some(StateTag {
            or_not: Message::StateMirroredOrNot,
            only: Message::StateMirroredOnly,
            text: color::MIRRORED_STATE,
            fill: color::MIRRORED_STATE_BG,
            hovered: color::MIRRORED_STATE_HOVER,
        }),
        ItemState::Unidentified => None,
    }
}

/// A main value of the base: the DPS of a weapon, of each kind it deals, or a defence of an
/// armour piece or a shield.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaseValue {
    Dps(DamageKind, u32),
    Armour(u32),
    Evasion(u32),
    EnergyShield(u32),
    /// The chance to block, in percent.
    Block(u32),
    /// The defence of a runeforged base, without attribute.
    RunicWard(u32),
    /// A property of a waystone, in percent but for the revives.
    Waystone(WaystoneProperty, i32),
}

impl BaseValue {
    fn label(self) -> Message {
        match self {
            Self::Dps(DamageKind::Physical, _) => Message::DpsPhysical,
            Self::Dps(DamageKind::Elemental, _) => Message::DpsElemental,
            Self::Dps(DamageKind::Chaos, _) => Message::DpsChaos,
            Self::Armour(_) => Message::DefenceArmour,
            Self::Evasion(_) => Message::DefenceEvasion,
            Self::EnergyShield(_) => Message::DefenceEnergyShield,
            Self::Block(_) => Message::DefenceBlock,
            Self::RunicWard(_) => Message::DefenceRunicWard,
            Self::Waystone(property, _) => match property {
                WaystoneProperty::Revives => Message::WaystoneRevives,
                WaystoneProperty::ItemRarity => Message::WaystoneItemRarity,
                WaystoneProperty::PackSize => Message::WaystonePackSize,
                WaystoneProperty::MonsterRarity => Message::WaystoneMonsterRarity,
                WaystoneProperty::MonsterEffectiveness => Message::WaystoneMonsterEffectiveness,
                WaystoneProperty::DropChance => Message::WaystoneDropChance,
            },
        }
    }

    /// The name of the game of a label written short, for its tooltip: the revives, and in
    /// French the drop chance of a waystone.
    fn full_label(self) -> Option<Message> {
        match self {
            Self::Waystone(WaystoneProperty::Revives, _) => Some(Message::WaystoneRevivesFull),
            Self::Waystone(WaystoneProperty::DropChance, _) => {
                Some(Message::WaystoneDropChanceFull)
            }
            _ => None,
        }
    }

    /// The color of the label: the one of the attribute a defence comes from, as in the
    /// game; a sky blue of its own for the Runic Ward, which comes from none.
    fn label_color(self) -> Color32 {
        match self {
            Self::Armour(_) => color::STRENGTH,
            Self::Evasion(_) => color::DEXTERITY,
            Self::EnergyShield(_) => color::INTELLIGENCE,
            Self::RunicWard(_) => color::RUNIC_WARD,
            Self::Dps(..) | Self::Block(_) | Self::Waystone(..) => color::WEAK,
        }
    }

    fn number(self) -> String {
        match self {
            Self::Block(percent) => format!("{percent}%"),
            // As the game writes it: `+11%`.
            Self::Waystone(property, value) if property.is_percent() => format!("{value:+}%"),
            Self::Waystone(_, value) => value.to_string(),
            Self::Dps(_, value)
            | Self::Armour(value)
            | Self::Evasion(value)
            | Self::EnergyShield(value)
            | Self::RunicWard(value) => value.to_string(),
        }
    }
}

/// A fact of the item, under its base type.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Fact {
    BaseValues(Vec<BaseValue>),
    Skills(String),
    /// Each part with its color.
    Requirements(Vec<(String, Color32)>),
    Tags(Vec<(String, TagKind)>),
}

pub struct ItemBlock<'a> {
    item: &'a Item,
    /// What the application knows of its modifier lines.
    facts: LineFacts<'a>,
    i18n: &'a Translator,
    /// The trade site has a search for this item.
    tradable: bool,
    /// The stats are unfolded.
    stats_open: bool,
    /// The base type its search looks for, a filter.
    base_type: Option<&'a str>,
    /// The filters of the search, as the player set them.
    filters: &'a mut Filters,
}

impl<'a> ItemBlock<'a> {
    #[must_use]
    pub fn new(
        item: &'a Item,
        facts: LineFacts<'a>,
        filters: &'a mut Filters,
        i18n: &'a Translator,
    ) -> Self {
        Self {
            item,
            facts,
            i18n,
            tradable: false,
            stats_open: false,
            base_type: None,
            filters,
        }
    }

    /// Unfolds the stats, with the base type its search looks for.
    #[must_use]
    pub fn stats(mut self, open: bool, base_type: Option<&'a str>) -> Self {
        self.stats_open = open;
        self.base_type = base_type;
        self
    }

    /// Shows the Check on trade pill: the trade site has a search for this item.
    #[must_use]
    pub fn tradable(mut self, tradable: bool) -> Self {
        self.tradable = tradable;
        self
    }

    /// Draws the block at the top of `ui`, on its band B, its whole width; what the player
    /// did in it.
    pub fn show(mut self, ui: &mut Ui) -> Option<ItemBlockAction> {
        Frame::NONE
            .inner_margin(vec2(size::BAND_PADDING, PADDING_Y))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let trade_action = self.name_row(ui);
                ui.add_space(BASE_GAP);
                ui.add(Label::new(self.base_and_class()).wrap());
                if let Some(state) = self.facts(ui) {
                    self.filters.toggle_state(state);
                }
                let stats_action = self.stats_section(ui);
                trade_action.or(stats_action)
            })
            .inner
    }

    /// The name, wrapped beside the Check on trade pill at the top right.
    fn name_row(&self, ui: &mut Ui) -> Option<ItemBlockAction> {
        let pill_label = self.i18n.text(Message::CheckOnTrade);
        let pill_width = if self.tradable {
            widgets::pill_size(ui, &pill_label).x + NAME_GAP
        } else {
            0.0
        };
        ui.horizontal_top(|ui| {
            let name = RichText::new(self.title())
                .font(text::item_name())
                .color(rarity_color(self.item.rarity()));
            ui.allocate_ui_with_layout(
                vec2((ui.available_width() - pill_width).max(0.0), 0.0),
                Layout::top_down(Align::Min),
                |ui| ui.add(Label::new(name).wrap()),
            );
            if !self.tradable {
                return None;
            }
            ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                widgets::control_pill(ui, &pill_label)
                    .clicked()
                    .then_some(ItemBlockAction::CheckOnTrade)
            })
            .inner
        })
        .inner
    }

    /// The name of the item; a waystone without its tier, which the line under it writes:
    /// `Burning Waystone`.
    fn title(&self) -> Cow<'_, str> {
        let title = self.item.title();
        if self.waystone_base().is_some()
            && let Some((name, _)) = item::without_tier(self.item.language(), title)
        {
            return Cow::Owned(name);
        }
        Cow::Borrowed(title)
    }

    /// The base and the tier of a waystone whose base is known: `Waystone` of tier 12.
    fn waystone_base(&self) -> Option<(&str, u32)> {
        let base_type = self.base_type.filter(|_| self.item.is_waystone())?;
        item::base_and_tier(self.item.language(), base_type)
    }

    /// The base type of a rare or unique item, then its class in gray: "Wanderer Coat · Body
    /// Armours"; the class alone for the other items, named by their base. A waystone, of any
    /// rarity, writes its base and its tier, in its color, without its class, which only repeats
    /// it: "Waystone · T12". A gem writes its level after its class, in the color of its level:
    /// "Skill Gems · Level 2".
    fn base_and_class(&self) -> LayoutJob {
        let mut job = LayoutJob::default();
        let font = text::item_line();
        if let Some((name, tier)) = self.waystone_base() {
            append(&mut job, name, &font, color::TEXT);
            append(&mut job, SEPARATOR, &font, color::WEAK);
            append(
                &mut job,
                &format!("T{tier}"),
                &text::base_value(),
                waystone_tier_color(tier),
            );
            return job;
        }
        if let Some(base_type) = self.base_line() {
            append(&mut job, base_type, &font, color::TEXT);
            append(&mut job, SEPARATOR, &font, color::WEAK);
        }
        append(&mut job, self.item.class(), &font, color::WEAK);
        if let Some(level) = self.item.skill_gem_level() {
            append(&mut job, SEPARATOR, &font, color::WEAK);
            append(
                &mut job,
                &self.i18n.gem_level(level),
                &text::base_value(),
                gem_level_color(level),
            );
        }
        job
    }

    /// The facts of the item, those it has, 6 px apart; the state whose tag the player
    /// clicked.
    fn facts(&self, ui: &mut Ui) -> Option<ItemState> {
        let mut clicked = None;
        let mut previous = None;
        for fact in self.fact_list() {
            ui.add_space(match (&previous, &fact) {
                (None, _) => FACTS_GAP,
                (Some(Fact::BaseValues(_)), Fact::Requirements(_)) => VALUES_REQUIREMENTS_GAP,
                _ => FACT_GAP,
            });
            match &fact {
                Fact::BaseValues(values) => self.base_values_line(ui, values),
                Fact::Skills(skills) => {
                    ui.add(
                        Label::new(
                            RichText::new(skills)
                                .font(text::item_line())
                                .color(color::TEXT),
                        )
                        .wrap(),
                    );
                }
                Fact::Requirements(parts) => {
                    ui.add(Label::new(self.requirements_line(parts)).wrap());
                }
                Fact::Tags(tags) => clicked = self.tags_line(ui, tags),
            }
            previous = Some(fact);
        }
        clicked
    }

    /// The tags, wrapped when needed; the state whose tag the player clicked.
    fn tags_line(&self, ui: &mut Ui, tags: &[(String, TagKind)]) -> Option<ItemState> {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = TAG_SPACING;
            let mut clicked = None;
            for (label, kind) in tags {
                match *kind {
                    TagKind::Plain => {
                        tag(ui, label, Sense::hover(), |_| (color::CONTROL, color::TEXT));
                    }
                    TagKind::Top(tint) => {
                        tag(ui, label, Sense::hover(), |_| (color::CONTROL, tint));
                    }
                    TagKind::State(state, state_tag) => {
                        let lit = self.filters.is_lit(state);
                        let response = tag(ui, label, Sense::click(), |response| {
                            // The keyboard focus looks like the pointer, as on the pills.
                            match (lit, response.hovered() || response.has_focus()) {
                                (false, false) => (color::CONTROL, color::WEAK),
                                (false, true) => (color::CONTROL_LINE, color::TEXT),
                                (true, false) => (state_tag.fill, state_tag.text),
                                (true, true) => (state_tag.hovered, state_tag.text),
                            }
                        });
                        response
                            .widget_info(|| WidgetInfo::selected(Role::CheckBox, true, lit, label));
                        let tooltip = if lit {
                            state_tag.only
                        } else {
                            state_tag.or_not
                        };
                        if response
                            .on_hover_cursor(CursorIcon::PointingHand)
                            .on_hover_text(self.i18n.text(tooltip))
                            .clicked()
                        {
                            clicked = Some(state);
                        }
                    }
                }
            }
            clicked
        })
        .inner
    }

    /// The facts the item has, in their order.
    fn fact_list(&self) -> Vec<Fact> {
        let base_values = self.base_values();
        let tags = self.tags();
        [
            (!base_values.is_empty()).then_some(Fact::BaseValues(base_values)),
            self.skills().map(Fact::Skills),
            self.requirements().map(Fact::Requirements),
            (!tags.is_empty()).then_some(Fact::Tags(tags)),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// The base values, each label in gray or in the color of its attribute and its number
    /// in white bold, wrapped when needed.
    fn base_values_line(&self, ui: &mut Ui, values: &[BaseValue]) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = VALUE_SPACING;
            for &value in values {
                let mut job = LayoutJob::default();
                append(
                    &mut job,
                    &self.i18n.text(value.label()),
                    &text::item_line(),
                    value.label_color(),
                );
                job.append(
                    &value.number(),
                    VALUE_NUMBER_GAP,
                    TextFormat::simple(text::base_value(), color::TEXT),
                );
                let response = ui.add(Label::new(job).extend());
                if let Some(full) = value.full_label() {
                    let (short, full) = (self.i18n.text(value.label()), self.i18n.text(full));
                    if full != short {
                        response.on_hover_text(full);
                    }
                }
            }
        });
    }

    /// The pill of the stats and the number of active filters, and the stats once unfolded;
    /// "Show filters" for an item without modifiers, nothing when it has no filter either.
    fn stats_section(&mut self, ui: &mut Ui) -> Option<ItemBlockAction> {
        let stats = Stats::new(self.item, self.facts, self.base_type);
        if stats.is_empty() {
            return None;
        }
        ui.add_space(STATS_PILL_GAP);
        let label = match (item_stats::count(self.item), self.stats_open) {
            (0, false) => self.i18n.text(Message::ShowFilters),
            (0, true) => self.i18n.text(Message::HideFilters),
            (_, true) => self.i18n.text(Message::HideStats),
            (count, false) => self.i18n.show_stats(count),
        };
        let active = stats.active_filters(self.filters);
        let action = ui
            .horizontal(|ui| {
                let clicked = widgets::control_pill(ui, &label).clicked();
                if active > 0 {
                    ui.add_space(FILTERS_GAP);
                    ui.label(
                        RichText::new(self.i18n.active_filters(active))
                            .font(text::pill())
                            .color(color::ACCENT),
                    );
                }
                clicked
            })
            .inner
            .then_some(ItemBlockAction::ToggleStats);
        if self.stats_open {
            ui.add_space(STATS_GAP);
            stats.show(ui, self.filters, self.i18n);
        }
        action
    }

    /// The base type under the name of a rare or unique item, as the game writes it, with
    /// "Exceptional"; the other items are named by their base.
    fn base_line(&self) -> Option<&str> {
        self.item.name().map(|_| self.item.written_base_type())
    }

    /// The tags, in the order of the specifications: the states, then the numbers.
    fn tags(&self) -> Vec<(String, TagKind)> {
        let item = self.item;
        let i18n = self.i18n;
        let mut tags: Vec<(String, TagKind)> = STATE_TAGS
            .into_iter()
            .filter(|&(state, _)| item.has_state(state))
            .map(|(state, message)| {
                let kind =
                    state_tag(state).map_or(TagKind::Plain, |tag| TagKind::State(state, tag));
                (i18n.text(message), kind)
            })
            .collect();
        let numbers = [
            (NumberTag::Quality, item.quality()),
            (NumberTag::Sockets, item.sockets()),
            (NumberTag::ItemLevel, item.item_level()),
        ];
        for (number, value) in numbers {
            if let Some(value) = value {
                let kind = match top_color(item, number, value) {
                    Some(tint) => TagKind::Top(tint),
                    None => TagKind::Plain,
                };
                tags.push((i18n.tag(number, value), kind));
            }
        }
        if let Some(stack) = item.stack() {
            tags.push((i18n.stack_tag(stack.size, stack.max), TagKind::Plain));
        }
        tags
    }

    /// The DPS of a weapon, each kind it deals; the defences of an armour piece or a shield;
    /// or the properties of a waystone, in the order of the item.
    fn base_values(&self) -> Vec<BaseValue> {
        let waystone = self.item.waystone_properties();
        if !waystone.is_empty() {
            return waystone
                .into_iter()
                .map(|found| BaseValue::Waystone(found.property, found.value))
                .collect();
        }
        if let Some(weapon) = self.item.weapon() {
            let dps = weapon.dps();
            return [
                (DamageKind::Physical, dps.physical),
                (DamageKind::Elemental, dps.elemental),
                (DamageKind::Chaos, dps.chaos),
            ]
            .into_iter()
            .filter_map(|(kind, value)| Some(BaseValue::Dps(kind, rounded(value)?)))
            .collect();
        }
        let defences = self.item.defences();
        [
            defences.armour.map(BaseValue::Armour),
            defences.evasion.map(BaseValue::Evasion),
            defences.energy_shield.map(BaseValue::EnergyShield),
            defences.block.map(BaseValue::Block),
            defences.runic_ward.map(BaseValue::RunicWard),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// The skills the item grants, e.g. a staff.
    fn skills(&self) -> Option<String> {
        let skills = self.item.granted_skills();
        (!skills.is_empty()).then(|| self.i18n.granted_skill(&skills.join(SEPARATOR)))
    }

    /// The level and the attributes the item requires, each in its color.
    fn requirements(&self) -> Option<Vec<(String, Color32)>> {
        requirement_parts(self.item.requirements(), self.i18n)
    }

    /// The requirements line, in gray with each attribute in its color: "Requires Level 65 ·
    /// 145 Dex".
    fn requirements_line(&self, parts: &[(String, Color32)]) -> LayoutJob {
        requirements_job(parts, self.i18n, &text::item_line())
    }
}

/// The level and the attributes of `requirements`, each in its color; none without any.
pub(super) fn requirement_parts(
    requirements: Requirements,
    i18n: &Translator,
) -> Option<Vec<(String, Color32)>> {
    let parts: Vec<(String, Color32)> = [
        (Requirement::Level, requirements.level, color::WEAK),
        (
            Requirement::Strength,
            requirements.strength,
            color::STRENGTH,
        ),
        (
            Requirement::Dexterity,
            requirements.dexterity,
            color::DEXTERITY,
        ),
        (
            Requirement::Intelligence,
            requirements.intelligence,
            color::INTELLIGENCE,
        ),
    ]
    .into_iter()
    .filter_map(|(part, value, tint)| Some((i18n.requirement(part, value?), tint)))
    .collect();
    (!parts.is_empty()).then_some(parts)
}

/// The requirements line of `parts` in `font`, in gray with each attribute in its color:
/// "Requires Level 65 · 145 Dex".
pub(super) fn requirements_job(
    parts: &[(String, Color32)],
    i18n: &Translator,
    font: &FontId,
) -> LayoutJob {
    let requires = i18n.requires();
    let mut job = LayoutJob::default();
    append(&mut job, &requires.before, font, color::WEAK);
    for (index, (part, tint)) in parts.iter().enumerate() {
        if index > 0 {
            append(&mut job, SEPARATOR, font, color::WEAK);
        }
        append(&mut job, part, font, *tint);
    }
    append(&mut job, &requires.after, font, color::WEAK);
    job
}

/// Adds `text` to `job`, in `font` and `tint`.
fn append(job: &mut LayoutJob, text: &str, font: &FontId, tint: Color32) {
    job.append(text, 0.0, TextFormat::simple(font.clone(), tint));
}

/// A tag: its label on a small block, sensing `sense`, its background and its text in the
/// `colors` chosen from its response (under the pointer or not).
fn tag(
    ui: &mut Ui,
    label: &str,
    sense: Sense,
    colors: impl FnOnce(&Response) -> (Color32, Color32),
) -> Response {
    let galley = widgets::text_line(ui, label, text::tag());
    let (rect, response) = ui.allocate_exact_size(galley.size() + 2.0 * TAG_PADDING, sense);
    let (fill, tint) = colors(&response);
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), fill);
    painter.galley(rect.min + TAG_PADDING, galley, tint);
    response
}

/// The color of the name of an item of `rarity`, as in the game.
pub(super) fn rarity_color(rarity: Rarity) -> Color32 {
    match rarity {
        Rarity::Normal => color::NORMAL,
        Rarity::Magic => color::MAGIC,
        Rarity::Rare => color::RARE,
        Rarity::Unique => color::UNIQUE,
        Rarity::Currency => color::CURRENCY,
        Rarity::Gem => color::GEM,
    }
}

/// The color of the tier of a waystone, which tells at a glance how dangerous the map is:
/// tiers 1 to 5 white, 6 to 10 the yellow of a warning, 11 to 15 the orange of the best tier
/// of a modifier, 16 (a corrupted waystone raised one tier) the corrupted red.
pub(super) fn waystone_tier_color(tier: u32) -> Color32 {
    match tier {
        ..=5 => color::TEXT,
        6..=10 => color::WARNING,
        11..=15 => color::TIER_1,
        _ => color::VAAL,
    }
}

/// The color of the level of a gem, as the tier of a waystone: up to 19 white, 20 (the highest
/// from the gem) the orange of tier 1, 21 (raised by the corruption) the corrupted red.
pub(super) fn gem_level_color(level: u32) -> Color32 {
    match level {
        ..TOP_GEM_LEVEL => color::TEXT,
        TOP_GEM_LEVEL => color::TIER_1,
        _ => color::VAAL,
    }
}

/// The color of the tag of `number` of `item` at its top, in the colors of the level of a gem:
/// for a gem only, its quality of 20 % the orange of tier 1, above it (corrupted) the corrupted
/// red; its 5 sockets, the most, the orange. None for the other items, where a quality of 20 %
/// is usual.
fn top_color(item: &Item, number: NumberTag, value: u32) -> Option<Color32> {
    if !item.is_skill_gem() {
        return None;
    }
    match number {
        NumberTag::Quality if value > TOP_GEM_QUALITY => Some(color::VAAL),
        NumberTag::Quality if value == TOP_GEM_QUALITY => Some(color::TIER_1),
        NumberTag::Sockets if value >= MAX_GEM_SOCKETS => Some(color::TIER_1),
        _ => None,
    }
}

/// A DPS rounded to a whole number; none when it rounds to zero, or past [`MAX_DPS`].
fn rounded(dps: f64) -> Option<u32> {
    let rounded = dps.round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive DPS under MAX_DPS"
    )]
    (1.0..=MAX_DPS).contains(&rounded).then_some(rounded as u32)
}

#[cfg(test)]
mod tests;
