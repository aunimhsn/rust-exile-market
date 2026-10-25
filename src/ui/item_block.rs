//! The item block: the name of the copied item in its rarity color, the Check on trade pill,
//! its base type, its class and tags, the summary of a weapon or a granted skill, and its
//! requirements.

use egui::{
    Align, CornerRadius, Frame, Label, Layout, RichText, Sense, Ui, Vec2, WidgetText, vec2,
};

use super::theme::{color, size, text};
use super::widgets;
use crate::i18n::{DpsKind, Message, NumberTag, Requirement, Translator};
use crate::item::{Item, ItemState, Rarity};

/// The space above and under the block.
const PADDING_Y: f32 = 12.0;
/// The space between the name and the Check on trade pill.
const NAME_GAP: f32 = 12.0;
/// The space between the name and the base type.
const BASE_GAP: f32 = 2.0;
/// The space above the class and tags, and above the requirements.
const LINE_GAP: f32 = 6.0;
/// The space above the summary.
const SUMMARY_GAP: f32 = 8.0;
/// The space between the tags, across and down when they wrap.
const TAG_SPACING: Vec2 = vec2(6.0, 5.0);
/// The space around the text of a tag.
const TAG_PADDING: Vec2 = vec2(7.0, 1.0);
/// The separator of the parts of the summary and of the requirements.
const SEPARATOR: &str = " · ";

/// What the player did in the item block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemBlockAction {
    CheckOnTrade,
}

/// The style of a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TagStyle {
    Plain,
    Corrupted,
    Tier1,
}

pub struct ItemBlock<'a> {
    item: &'a Item,
    i18n: &'a Translator,
    /// The trade site has a search for this item.
    tradable: bool,
}

impl<'a> ItemBlock<'a> {
    #[must_use]
    pub fn new(item: &'a Item, i18n: &'a Translator) -> Self {
        Self {
            item,
            i18n,
            tradable: false,
        }
    }

    /// Shows the Check on trade pill: the trade site has a search for this item.
    #[must_use]
    pub fn tradable(mut self, tradable: bool) -> Self {
        self.tradable = tradable;
        self
    }

    /// Draws the block at the top of `ui`, its whole width; what the player did in it.
    pub fn show(self, ui: &mut Ui) -> Option<ItemBlockAction> {
        Frame::NONE
            .fill(color::BAND_B)
            .inner_margin(vec2(size::BAND_PADDING, PADDING_Y))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                let action = self.name_row(ui);
                if let Some(base_type) = self.base_line() {
                    ui.add_space(BASE_GAP);
                    ui.add(Label::new(line(base_type, color::TEXT)).wrap());
                }
                ui.add_space(LINE_GAP);
                self.class_and_tags(ui);
                if let Some(summary) = self.summary() {
                    ui.add_space(SUMMARY_GAP);
                    ui.add(Label::new(line(&summary, color::TEXT)).wrap());
                }
                if let Some(requirements) = self.requirements() {
                    ui.add_space(LINE_GAP);
                    ui.add(
                        Label::new(
                            RichText::new(requirements)
                                .font(text::tag())
                                .color(color::WEAK),
                        )
                        .wrap(),
                    );
                }
                action
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

    /// The base type under the name of a rare or unique item; the other items are named by
    /// their base.
    fn base_line(&self) -> Option<&str> {
        self.item.name().map(|_| self.item.base_type())
    }

    /// The class, then the tags, wrapped when needed.
    fn class_and_tags(&self, ui: &mut Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = TAG_SPACING;
            ui.label(
                RichText::new(self.item.class())
                    .font(text::item_line())
                    .color(color::WEAK),
            );
            for (label, style) in self.tags() {
                tag(ui, &label, style);
            }
        });
    }

    /// The tags, in the order of the specifications: the states, then the numbers.
    fn tags(&self) -> Vec<(String, TagStyle)> {
        let item = self.item;
        let i18n = self.i18n;
        let mut tags = Vec::new();
        for state in item.states() {
            let (message, style) = match state {
                ItemState::Corrupted => (Message::TagCorrupted, TagStyle::Corrupted),
                ItemState::TwiceCorrupted => (Message::TagTwiceCorrupted, TagStyle::Corrupted),
                ItemState::Sanctified => (Message::TagSanctified, TagStyle::Plain),
                ItemState::Fractured => (Message::TagFractured, TagStyle::Plain),
                ItemState::Mirrored => (Message::TagMirrored, TagStyle::Plain),
                ItemState::Unidentified => (Message::TagUnidentified, TagStyle::Plain),
            };
            tags.push((i18n.text(message), style));
        }
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
        let tier_1 = item.tier_1_affixes();
        if tier_1 > 0 && matches!(item.rarity(), Rarity::Magic | Rarity::Rare) {
            let count = u32::try_from(tier_1).unwrap_or(u32::MAX);
            tags.push((i18n.tag(NumberTag::Tier1, count), TagStyle::Tier1));
        }
        tags
    }

    /// The DPS of a weapon, each kind it deals, or the skill an item grants.
    fn summary(&self) -> Option<String> {
        if let Some(weapon) = self.item.weapon() {
            let dps = weapon.dps();
            let parts: Vec<String> = [
                (DpsKind::Physical, dps.physical),
                (DpsKind::Elemental, dps.elemental),
                (DpsKind::Chaos, dps.chaos),
            ]
            .into_iter()
            .filter_map(|(kind, value)| Some(self.i18n.dps(kind, rounded(value)?)))
            .collect();
            return (!parts.is_empty()).then(|| parts.join(SEPARATOR));
        }
        self.item
            .granted_skills()
            .first()
            .map(|skill| self.i18n.granted_skill(skill))
    }

    /// The level and the attributes the item requires.
    fn requirements(&self) -> Option<String> {
        let requirements = self.item.requirements();
        let parts: Vec<String> = [
            (Requirement::Level, requirements.level),
            (Requirement::Strength, requirements.strength),
            (Requirement::Dexterity, requirements.dexterity),
            (Requirement::Intelligence, requirements.intelligence),
        ]
        .into_iter()
        .filter_map(|(part, value)| Some(self.i18n.requirement(part, value?)))
        .collect();
        (!parts.is_empty()).then(|| self.i18n.requires(&parts.join(SEPARATOR)))
    }
}

/// A line of the item block in `tint`.
fn line(text: &str, tint: egui::Color32) -> WidgetText {
    RichText::new(text)
        .font(text::item_line())
        .color(tint)
        .into()
}

/// A tag: its label on a small block.
fn tag(ui: &mut Ui, label: &str, style: TagStyle) {
    let (fill, tint) = match style {
        TagStyle::Plain => (color::CONTROL, color::TEXT),
        TagStyle::Corrupted => (color::CORRUPTED_TAG, color::CORRUPTED_TAG_TEXT),
        TagStyle::Tier1 => (color::CONTROL, color::TIER_1),
    };
    let galley = widgets::text_line(ui, label, text::tag());
    let (rect, _) = ui.allocate_exact_size(galley.size() + 2.0 * TAG_PADDING, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(size::RADIUS), fill);
    painter.galley(rect.min + TAG_PADDING, galley, tint);
}

/// The color of the name of an item of `rarity`, as in the game.
fn rarity_color(rarity: Rarity) -> egui::Color32 {
    match rarity {
        Rarity::Normal => color::NORMAL,
        Rarity::Magic => color::MAGIC,
        Rarity::Rare => color::RARE,
        Rarity::Unique => color::UNIQUE,
        Rarity::Currency => color::CURRENCY,
        Rarity::Gem => color::GEM,
    }
}

/// A DPS rounded to a whole number; none when it rounds to zero.
fn rounded(dps: f64) -> Option<u32> {
    let rounded = dps.round();
    // The DPS of a weapon stays far under the largest `u32`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive DPS under a million"
    )]
    (1.0..1e9).contains(&rounded).then_some(rounded as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::CopiedItem;
    use crate::language::Language;

    fn parse(text: &str) -> Item {
        Item::parse(&CopiedItem::from_text(text).expect("an item")).expect("read")
    }

    fn block_texts(
        text: &str,
        language: Language,
    ) -> (Vec<String>, Option<String>, Option<String>) {
        let item = parse(text);
        let i18n = Translator::new(language);
        let block = ItemBlock::new(&item, &i18n);
        let tags = block.tags().into_iter().map(|(label, _)| label).collect();
        (tags, block.summary(), block.requirements())
    }

    #[test]
    fn a_rare_weapon_shows_its_tags_dps_and_requirements() {
        let (tags, summary, requirements) = block_texts(
            include_str!("../item/test-items/en-rare-crossbow-all-modifiers.txt"),
            Language::English,
        );

        assert!(tags.contains(&"Q 20%".to_owned()), "{tags:?}");
        assert_eq!(tags.last().map(String::as_str), Some("2 T1"));
        assert_eq!(
            summary.as_deref(),
            Some("Physical DPS 288 · Elemental DPS 267")
        );
        assert!(
            requirements.is_some_and(|line| line.starts_with("Requires Level ")),
            "a requirements line"
        );
    }

    #[test]
    fn a_french_item_shows_its_texts_in_the_language_of_the_interface() {
        let (tags, summary, requirements) = block_texts(
            include_str!("../item/test-items/fr-rare-staff.txt"),
            Language::French,
        );

        assert!(tags.iter().any(|tag| tag.starts_with("niv.")), "{tags:?}");
        assert!(summary.is_some_and(|line| line.starts_with("Octroie l'aptitude")));
        assert_eq!(requirements.as_deref(), Some("Requiert niveau 64 · 87 Int"));
    }

    #[test]
    fn a_dps_under_one_isnt_shown() {
        assert_eq!(rounded(0.4), None);
        assert_eq!(rounded(244.6), Some(245));
    }
}
