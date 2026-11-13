use super::*;
use crate::item::CopiedItem;

/// A frame of egui with `events`, `content` drawing it.
fn egui_frame(
    ctx: &Context,
    time: f64,
    events: Vec<egui::Event>,
    content: impl FnMut(&mut egui::Ui),
) {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            vec2(420.0, 300.0),
        )),
        time: Some(time),
        events,
        ..Default::default()
    };
    crate::ui::fixtures::run_frame(ctx, input, content);
}

fn key(key: Key) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

/// After Tab (the map of the game), Tab then Space (its dodge), then 1 (a flask): whether a
/// widget has the keyboard, whether the button was pressed, the text of the field; only Esc
/// left to the frames when `keep`.
fn after_game_keys(keep: bool) -> (bool, bool, String) {
    let ctx = Context::default();
    let mut text = String::new();
    let mut clicked = false;
    let frames = [
        vec![],
        vec![key(Key::Tab)],
        vec![key(Key::Tab), key(Key::Space)],
        vec![key(Key::Num1), egui::Event::Text("1".to_owned())],
    ];
    for (at, events) in (0_u32..).zip(frames) {
        egui_frame(&ctx, f64::from(at) * 0.2, events, |ui| {
            if keep {
                keep_only_esc(ui);
            }
            clicked |= ui.button("Search").clicked();
            ui.text_edit_singleline(&mut text);
        });
    }
    let focused = ctx.memory(egui::Memory::focused).is_some();
    (focused, clicked, text)
}

#[test]
fn the_keys_of_the_game_neither_move_the_keyboard_nor_press_nor_type() {
    assert_eq!(after_game_keys(true), (false, false, String::new()));
    // What egui does with them otherwise.
    assert_ne!(after_game_keys(false), (false, false, String::new()));
}

#[test]
fn esc_leaves_a_field_before_the_frame_reads_it() {
    let ctx = Context::default();
    let mut text = String::new();
    let field = egui::Id::NULL.with("field");
    let mut draw = |ui: &mut egui::Ui| {
        ui.add(egui::TextEdit::singleline(&mut text).id(field));
    };
    egui_frame(&ctx, 0.0, vec![], &mut draw);
    ctx.memory_mut(|memory| memory.request_focus(field));
    egui_frame(&ctx, 0.2, vec![], &mut draw);
    assert!(ctx.text_edit_focused());

    // So the window reads whether a field was edited at the last frame
    // (`Activation::escape_closes`): this one already left it, its Esc still there.
    let mut seen = None;
    egui_frame(&ctx, 0.4, vec![key(Key::Escape)], |ui| {
        seen = Some((
            ui.ctx().text_edit_focused(),
            ui.input(|input| input.key_pressed(Key::Escape)),
        ));
        draw(ui);
    });

    assert_eq!(seen, Some((false, true)));
}

#[test]
fn paints_while_hidden_only_for_the_window_over_the_game_and_a_hidden_main_window() {
    assert!(paint_while_hidden(true, Some(false)));
    assert!(!paint_while_hidden(true, Some(true)));
    assert!(!paint_while_hidden(true, None));
    assert!(!paint_while_hidden(false, Some(false)));
}

#[test]
fn the_hotkey_on_the_item_shown_keeps_it_as_it_is() {
    let ring = CopiedItem::from_text("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring")
        .expect("an item");
    let amulet = CopiedItem::from_text("Item Class: Amulets\nRarity: Rare\nDoom Idol\nGold Amulet")
        .expect("an item");

    assert!(copied_again(true, Some(&ring), &ring));
    assert!(!copied_again(true, Some(&ring), &amulet));
    assert!(!copied_again(true, None, &ring));
    // Copied with Ctrl+C after another text: read again.
    assert!(!copied_again(false, Some(&ring), &ring));
}

#[test]
fn an_item_that_comes_back_forgets_its_stat_filters_only_when_its_lines_moved() {
    // The hybrid of a magic mace joined by the names arrived since it was left.
    assert!(lines_moved(Some(&[1, 1, 1]), Some(&[2, 1])));
    assert!(!lines_moved(Some(&[2, 1]), Some(&[2, 1])));
    // An item not remembered, or the item shown copied again.
    assert!(!lines_moved(None, Some(&[2, 1])));
}

#[test]
fn the_gear_opens_the_settings_then_closes_them() {
    assert_eq!(View::Item.toggled(View::Settings), View::Settings);
    assert_eq!(View::Settings.toggled(View::Settings), View::Item);
}

#[test]
fn the_bug_opens_the_bug_report_and_the_gear_the_settings_in_its_place() {
    assert_eq!(View::Item.toggled(View::BugReport), View::BugReport);
    assert_eq!(View::BugReport.toggled(View::BugReport), View::Item);
    assert_eq!(View::BugReport.toggled(View::Settings), View::Settings);
    assert_eq!(View::Settings.toggled(View::BugReport), View::BugReport);
}

#[test]
fn a_copied_item_closes_the_bug_report() {
    let copied = ClipboardEvent::Copied {
        item: CopiedItem::from_text("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring")
            .unwrap(),
        hotkey: None,
    };

    assert_eq!(View::BugReport.after(&copied), View::Item);
    assert_eq!(
        View::BugReport.after(&ClipboardEvent::Unreadable),
        View::BugReport
    );
}

#[test]
fn a_copied_item_closes_the_settings() {
    let copied = ClipboardEvent::Copied {
        item: CopiedItem::from_text("Item Class: Rings\nRarity: Rare\nWoe Loop\nRuby Ring")
            .unwrap(),
        hotkey: None,
    };

    assert_eq!(View::Settings.after(&copied), View::Item);
    assert_eq!(View::Item.after(&copied), View::Item);
}

#[test]
fn the_status_bar_tells_a_long_wait_rather_than_the_age_of_the_last_search() {
    use crate::trade::query::Query;
    use crate::trade::search::{Answer, NextSearch, Outcome};

    let now = Instant::now();
    let clock = SystemTime::UNIX_EPOCH + Duration::from_secs(600);
    let mut offers = Offers::default();
    assert_eq!(
        trade_status(&offers, SearchKind::Items, now, clock),
        TradeStatus::Idle
    );
    let search = Search {
        language: crate::language::Language::English,
        league: League::new("Standard"),
        query: Query::Exchange("divine".to_owned()),
    };
    let _ = offers.search(search, now);
    let answer = |seconds| Answer {
        outcome: Outcome::Exchange {
            offers: Vec::new(),
            total: 0,
        },
        next_search: NextSearch {
            items: now + Duration::from_secs(seconds),
            exchange: now + Duration::from_secs(10),
        },
    };
    offers.apply(
        answer(240),
        SystemTime::UNIX_EPOCH + Duration::from_secs(480),
    );

    assert_eq!(
        trade_status(&offers, SearchKind::Items, now, clock),
        TradeStatus::Waiting(Duration::from_secs(240))
    );
    assert_eq!(
        trade_status(&offers, SearchKind::Exchange, now, clock),
        TradeStatus::Searched(Duration::from_secs(120)),
        "the usual 10 s aren't told"
    );
}

#[test]
fn the_state_of_the_clipboard_keeps_the_view() {
    for event in [ClipboardEvent::Unreadable, ClipboardEvent::Readable] {
        assert_eq!(View::Settings.after(&event), View::Settings);
        assert_eq!(View::Item.after(&event), View::Item);
    }
}
