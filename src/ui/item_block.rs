//! The item block: the name of the copied item in its rarity color, the Check on trade pill,
//! its base type and class, then its facts (the main values of its base, the skills it
//! grants, its requirements, its tags), and the pill of its stats. A click on a state tag
//! (Corrupted, Fractured…) lights it: the offers of that state only.

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Color32, CornerRadius, CursorIcon, FontId, Frame, Label, Layout, Response, RichText,
    Role, Sense, Ui, Vec2, WidgetInfo, vec2,
};

use super::item_stats::{self, Stats};
use super::theme::{color, size, text};
use super::widgets;
use crate::filters::Filters;
use crate::i18n::{Message, NumberTag, Requirement, Translator};
use crate::item::{DamageKind, Item, ItemState, Rarity};
use crate::roll_difference::CopiedLines;
use crate::tier_rolls::TierRolls;
use crate::trade::stat_filters::LineStats;

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
/// The separator of the base type and the class, of the skills and of the requirements.
const SEPARATOR: &str = " · ";
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
            Self::Dps(..) | Self::Block(_) => color::WEAK,
        }
    }

    fn number(self) -> String {
        match self {
            Self::Block(percent) => format!("{percent}%"),
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
    /// Its modifier lines, which tell the stats better lower.
    lines: &'a CopiedLines,
    /// The stats of its modifier lines, which tell those the trade site can search.
    stats: &'a LineStats,
    /// The rolls of every tier of a line of a prefix or suffix.
    tiers: &'a TierRolls,
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
        lines: &'a CopiedLines,
        stats: &'a LineStats,
        tiers: &'a TierRolls,
        filters: &'a mut Filters,
        i18n: &'a Translator,
    ) -> Self {
        Self {
            item,
            lines,
            stats,
            tiers,
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
            let name = RichText::new(self.item.title())
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

    /// The base type of a rare or unique item, then its class in gray: "Wanderer Coat · Body
    /// Armours"; the class alone for the other items, named by their base.
    fn base_and_class(&self) -> LayoutJob {
        let mut job = LayoutJob::default();
        let font = text::item_line();
        if let Some(base_type) = self.base_line() {
            append(&mut job, base_type, &font, color::TEXT);
            append(&mut job, SEPARATOR, &font, color::WEAK);
        }
        append(&mut job, self.item.class(), &font, color::WEAK);
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
                ui.add(Label::new(job).extend());
            }
        });
    }

    /// The pill of the stats and the number of active filters, and the stats once unfolded;
    /// "Show filters" for an item without modifiers, nothing when it has no filter either.
    fn stats_section(&mut self, ui: &mut Ui) -> Option<ItemBlockAction> {
        let stats = Stats::new(
            self.item,
            self.lines,
            self.stats,
            self.tiers,
            self.base_type,
        );
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
                tags.push((i18n.tag(number, value), TagKind::Plain));
            }
        }
        if let Some(stack) = item.stack() {
            tags.push((i18n.stack_tag(stack.size, stack.max), TagKind::Plain));
        }
        tags
    }

    /// The DPS of a weapon, each kind it deals; or the defences of an armour piece or a
    /// shield.
    fn base_values(&self) -> Vec<BaseValue> {
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
        let requirements = self.item.requirements();
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
        .filter_map(|(part, value, tint)| Some((self.i18n.requirement(part, value?), tint)))
        .collect();
        (!parts.is_empty()).then_some(parts)
    }

    /// The requirements line, in gray with each attribute in its color: "Requires Level 65 ·
    /// 145 Dex".
    fn requirements_line(&self, parts: &[(String, Color32)]) -> LayoutJob {
        let font = text::item_line();
        let (before, after) = self.i18n.requires_around();
        let mut job = LayoutJob::default();
        append(&mut job, &before, &font, color::WEAK);
        for (index, (part, tint)) in parts.iter().enumerate() {
            if index > 0 {
                append(&mut job, SEPARATOR, &font, color::WEAK);
            }
            append(&mut job, part, &font, *tint);
        }
        append(&mut job, &after, &font, color::WEAK);
        job
    }
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
mod tests {
    use super::*;
    use crate::item::CopiedItem;
    use crate::language::Language;

    /// The texts of the block of the item copied as `text`, with the interface in `language`.
    struct Texts {
        base_line: Option<String>,
        tags: Vec<String>,
        base_values: Vec<String>,
        skills: Option<String>,
        requirements: Option<String>,
    }

    fn texts(text: &str, language: Language) -> Texts {
        let item = Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read");
        let i18n = Translator::new(language);
        let mut filters = Filters::default();
        let lines = CopiedLines::default();
        let stats = LineStats::default();
        let tiers = TierRolls::default();
        let block = ItemBlock::new(&item, &lines, &stats, &tiers, &mut filters, &i18n);
        Texts {
            base_line: block.base_line().map(str::to_owned),
            tags: block.tags().into_iter().map(|(label, _)| label).collect(),
            base_values: block
                .base_values()
                .into_iter()
                .map(|value| format!("{} {}", i18n.text(value.label()), value.number()))
                .collect(),
            skills: block.skills(),
            requirements: block
                .requirements()
                .map(|parts| block.requirements_line(&parts).text),
        }
    }

    fn english(text: &str) -> Texts {
        texts(text, Language::English)
    }

    #[test]
    fn a_rare_weapon_shows_its_tags_in_order_its_dps_and_requirements() {
        let crossbow = english(include_str!(
            "../item/test-items/en-rare-crossbow-all-modifiers.txt"
        ));

        assert_eq!(crossbow.base_line.as_deref(), Some("Gemini Crossbow"));
        assert_eq!(
            crossbow.tags,
            ["Corrupted", "Fractured", "Q 20%", "2 sockets", "ilvl 80",]
        );
        assert_eq!(
            crossbow.base_values,
            ["Physical DPS 288", "Elemental DPS 267"]
        );
        assert_eq!(
            crossbow.requirements.as_deref(),
            Some("Requires Level 78 · 89 Str · 89 Dex")
        );
    }

    #[test]
    fn an_armour_piece_or_a_shield_shows_its_defences() {
        let focus = english(include_str!("../item/test-items/en-unique-focus.txt"));
        let shield = texts(
            include_str!("../item/test-items/fr-normal-exceptional-shield.txt"),
            Language::French,
        );

        assert_eq!(focus.tags, ["ilvl 81"]);
        assert_eq!(focus.base_line.as_deref(), Some("Crystal Focus"));
        assert_eq!(focus.base_values, ["Energy Shield 44"]);
        assert_eq!(shield.base_line, None, "named by its base");
        assert_eq!(
            shield.base_values,
            ["Armure 112", "Évasion 102", "Blocage 25%"]
        );
    }

    #[test]
    fn a_runeforged_base_ends_its_defences_with_its_runic_ward_in_sky_blue() {
        let english = english(include_str!(
            "../item/test-items/en-owner-rare-helmet-runeforged.txt"
        ));
        let french = texts(
            include_str!("../item/test-items/fr-rare-helmet-runeforged.txt"),
            Language::French,
        );

        assert_eq!(
            english.base_values,
            ["Armour 314", "Energy Shield 88", "Runic Ward 56"]
        );
        assert_eq!(
            french.base_values,
            [
                "Armure 314",
                "Bouclier d'énergie 88",
                "Sauvegarde runique 56"
            ]
        );
        assert_eq!(BaseValue::RunicWard(56).label_color(), color::RUNIC_WARD);
    }

    #[test]
    fn a_stack_has_its_tag_and_no_requirements() {
        let stack = texts(
            include_str!("../item/test-items/fr-currency-stack.txt"),
            Language::French,
        );

        assert_eq!(stack.tags, ["Pile 40/40"]);
        assert_eq!(stack.requirements, None);
        assert_eq!(stack.base_values, Vec::<String>::new());
    }

    #[test]
    fn the_base_line_of_an_exceptional_rare_keeps_its_word() {
        let staff =
            english("Item Class: Staves\nRarity: Rare\nDoom Song\nExceptional Chiming Staff");

        assert_eq!(
            staff.base_line.as_deref(),
            Some("Exceptional Chiming Staff")
        );
    }

    #[test]
    fn a_french_item_shows_its_texts_in_the_language_of_the_interface() {
        let staff = texts(
            include_str!("../item/test-items/fr-rare-staff.txt"),
            Language::French,
        );

        assert!(
            staff.tags.iter().any(|tag| tag.starts_with("niv.")),
            "{:?}",
            staff.tags
        );
        assert!(
            staff
                .skills
                .is_some_and(|line| line.starts_with("Octroie l'aptitude"))
        );
        assert_eq!(
            staff.requirements.as_deref(),
            Some("Requiert niveau 64 · 87 Int")
        );
    }

    #[test]
    fn the_state_tags_but_unidentified_are_filters() {
        let ring = Item::parse(
            &CopiedItem::from_text(
                "Item Class: Rings\nRarity: Rare\nRuby Ring\n--------\nItem Level: 81\n\
                 --------\nUnidentified\n--------\nTwice Corrupted",
            )
            .expect("an item"),
        )
        .expect("read");
        let i18n = Translator::new(Language::English);
        let mut filters = Filters::default();
        let (lines, line_stats, tiers) = (
            CopiedLines::default(),
            LineStats::default(),
            TierRolls::default(),
        );
        let block = ItemBlock::new(&ring, &lines, &line_stats, &tiers, &mut filters, &i18n);

        let kinds: Vec<(String, Option<ItemState>)> = block
            .tags()
            .into_iter()
            .map(|(label, kind)| match kind {
                TagKind::Plain => (label, None),
                TagKind::State(state, _) => (label, Some(state)),
            })
            .collect();
        assert_eq!(
            kinds[..2],
            [
                (
                    "Twice Corrupted".to_owned(),
                    Some(ItemState::TwiceCorrupted)
                ),
                ("Unidentified".to_owned(), None),
            ]
        );
    }

    #[test]
    fn a_state_tag_says_what_it_searches_gray_and_lit() {
        let tooltips = |language, state| {
            let i18n = Translator::new(language);
            let tag = state_tag(state).expect("a filter");
            (i18n.text(tag.or_not), i18n.text(tag.only))
        };
        let pair = |gray: &str, lit: &str| (gray.to_owned(), lit.to_owned());

        assert_eq!(
            tooltips(Language::English, ItemState::Corrupted),
            pair("Corrupted or not", "Corrupted offers only")
        );
        assert_eq!(
            tooltips(Language::English, ItemState::TwiceCorrupted),
            pair("Corrupted or not", "Twice corrupted offers only")
        );
        assert_eq!(
            tooltips(Language::French, ItemState::Mirrored),
            pair("Reflétées ou non", "Offres reflétées seulement")
        );
        assert_eq!(
            state_tag(ItemState::Fractured).map(|tag| tag.text),
            Some(color::FRACTURED)
        );
        assert_eq!(state_tag(ItemState::Unidentified), None);
    }

    #[test]
    fn a_dps_under_one_or_past_the_largest_isnt_shown() {
        assert_eq!(rounded(0.4), None);
        assert_eq!(rounded(244.6), Some(245));
        assert_eq!(rounded(2e6), None);
    }
}
