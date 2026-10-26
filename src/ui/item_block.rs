//! The item block: the name of the copied item in its rarity color, the Check on trade pill,
//! its base type and class, then its facts (the main values of its base, the skills it
//! grants, its requirements, its tags), and the pill of its stats.

use egui::text::{LayoutJob, TextFormat};
use egui::{
    Align, Color32, CornerRadius, FontId, Frame, Label, Layout, RichText, Sense, Ui, Vec2, vec2,
};

use super::item_stats::{self, Stats};
use super::theme::{color, size, text};
use super::widgets;
use crate::filters::Filters;
use crate::i18n::{Message, NumberTag, Requirement, Translator};
use crate::item::{DamageKind, Item, ItemState, Rarity};
use crate::prices::UniqueModifiers;

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

/// The style of a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagStyle {
    Plain,
    Corrupted,
}

/// The state tags, in the order of the specifications.
const STATE_TAGS: [(ItemState, Message, TagStyle); 6] = [
    (
        ItemState::Corrupted,
        Message::TagCorrupted,
        TagStyle::Corrupted,
    ),
    (
        ItemState::TwiceCorrupted,
        Message::TagTwiceCorrupted,
        TagStyle::Corrupted,
    ),
    (
        ItemState::Sanctified,
        Message::TagSanctified,
        TagStyle::Plain,
    ),
    (ItemState::Fractured, Message::TagFractured, TagStyle::Plain),
    (ItemState::Mirrored, Message::TagMirrored, TagStyle::Plain),
    (
        ItemState::Unidentified,
        Message::TagUnidentified,
        TagStyle::Plain,
    ),
];

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
        }
    }

    /// The color of the label: the one of the attribute a defence comes from, as in the
    /// game.
    fn label_color(self) -> Color32 {
        match self {
            Self::Armour(_) => color::STRENGTH,
            Self::Evasion(_) => color::DEXTERITY,
            Self::EnergyShield(_) => color::INTELLIGENCE,
            Self::Dps(..) | Self::Block(_) => color::WEAK,
        }
    }

    fn number(self) -> String {
        match self {
            Self::Block(percent) => format!("{percent}%"),
            Self::Dps(_, value)
            | Self::Armour(value)
            | Self::Evasion(value)
            | Self::EnergyShield(value) => value.to_string(),
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
    Tags(Vec<(String, TagStyle)>),
}

pub struct ItemBlock<'a> {
    item: &'a Item,
    i18n: &'a Translator,
    /// The trade site has a search for this item.
    tradable: bool,
    /// The stats are unfolded.
    stats_open: bool,
    /// The modifiers of the unique in the price database, the ranges of its rolls.
    unique_modifiers: Option<&'a UniqueModifiers>,
    /// The filters of the search, as the player set them.
    filters: &'a mut Filters,
}

impl<'a> ItemBlock<'a> {
    #[must_use]
    pub fn new(item: &'a Item, filters: &'a mut Filters, i18n: &'a Translator) -> Self {
        Self {
            item,
            i18n,
            tradable: false,
            stats_open: false,
            unique_modifiers: None,
            filters,
        }
    }

    /// Unfolds the stats, with the modifiers of the unique in the price database for the
    /// ranges of an item copied without them.
    #[must_use]
    pub fn stats(mut self, open: bool, unique_modifiers: Option<&'a UniqueModifiers>) -> Self {
        self.stats_open = open;
        self.unique_modifiers = unique_modifiers;
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
                self.facts(ui);
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

    /// The facts of the item, those it has, 6 px apart.
    fn facts(&self, ui: &mut Ui) {
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
                Fact::Tags(tags) => tags_line(ui, tags),
            }
            previous = Some(fact);
        }
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
        let stats = Stats::new(self.item, self.unique_modifiers);
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
    fn tags(&self) -> Vec<(String, TagStyle)> {
        let item = self.item;
        let i18n = self.i18n;
        let mut tags: Vec<(String, TagStyle)> = STATE_TAGS
            .into_iter()
            .filter(|&(state, ..)| item.has_state(state))
            .map(|(_, message, style)| (i18n.text(message), style))
            .collect();
        let numbers = [
            (NumberTag::Quality, item.quality()),
            (NumberTag::Sockets, item.sockets()),
            (NumberTag::ItemLevel, item.item_level()),
        ];
        for (number, value) in numbers {
            if let Some(value) = value {
                tags.push((i18n.tag(number, value), TagStyle::Plain));
            }
        }
        if let Some(stack) = item.stack() {
            tags.push((i18n.stack_tag(stack.size, stack.max), TagStyle::Plain));
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

/// The tags, wrapped when needed.
fn tags_line(ui: &mut Ui, tags: &[(String, TagStyle)]) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = TAG_SPACING;
        for (label, style) in tags {
            tag(ui, label, *style);
        }
    });
}

/// A tag: its label on a small block.
fn tag(ui: &mut Ui, label: &str, style: TagStyle) {
    let (fill, tint) = match style {
        TagStyle::Plain => (color::CONTROL, color::TEXT),
        TagStyle::Corrupted => (color::CORRUPTED_TAG, color::CORRUPTED_TAG_TEXT),
    };
    let galley = widgets::text_line(ui, label, text::tag());
    let (rect, _) = ui.allocate_exact_size(galley.size() + 2.0 * TAG_PADDING, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), fill);
    painter.galley(rect.min + TAG_PADDING, galley, tint);
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
        let block = ItemBlock::new(&item, &mut filters, &i18n);
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
    fn a_dps_under_one_or_past_the_largest_isnt_shown() {
        assert_eq!(rounded(0.4), None);
        assert_eq!(rounded(244.6), Some(245));
        assert_eq!(rounded(2e6), None);
    }
}
