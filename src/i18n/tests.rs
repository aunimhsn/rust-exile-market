use super::*;
use std::collections::BTreeSet;

fn file_ids(language: Language) -> BTreeSet<String> {
    let resource =
        FluentResource::try_new(messages(language).to_owned()).expect("the messages parse");
    resource
        .entries()
        .filter_map(|entry| match entry {
            fluent_syntax::ast::Entry::Message(message) => Some(message.id.name.to_owned()),
            _ => None,
        })
        .collect()
}

#[test]
#[expect(clippy::too_many_lines, reason = "one line per message")]
fn the_code_and_every_file_have_the_same_messages() {
    let code_ids: BTreeSet<String> = Message::ALL
        .iter()
        .map(|message| message.id())
        .chain([
            STATUS_BAR,
            LEAGUE_TOOLTIP,
            STATUS_AUTHOR,
            NEW_LEAGUE,
            UPDATE_AVAILABLE,
            STATUS_UPDATE,
            HOTKEY_TAKEN,
            HOTKEY_TAKEN_SETTINGS,
            BUG_REPORT_JOIN_ITEM,
            BUG_REPORT_LOG_SESSION,
            STATUS_PRICES,
            STATUS_PRICES_OFFLINE,
            PRICES_UNREACHABLE,
            AGE_NOW,
            AGE_MINUTES,
            WAIT_SECONDS,
            WAIT_MINUTES,
            AGE_HOURS,
            AGE_DAYS,
            DOWNLOAD_TITLE,
            TAG_QUALITY,
            TAG_SOCKETS,
            TAG_ITEM_LEVEL,
            TAG_STACK,
            GEM_LEVEL,
            GRANTED_SKILL,
            REQUIRES,
            REQUIREMENT_LEVEL,
            REQUIREMENT_STRENGTH,
            REQUIREMENT_DEXTERITY,
            REQUIREMENT_INTELLIGENCE,
            SHOW_STATS,
            AFFIX_FILL,
            AFFIX_FILL_OPEN,
            LORD_PREFIX,
            LORD_SUFFIX,
            ACTIVE_FILTERS,
            FILTER_ROLLS,
            HYBRID_WITH,
            HYBRID_LINES_AND,
            HYBRID_PART,
            FILTER_BASE_TYPE,
            FILTER_ITEM_CLASS,
            FILTER_ITEM_LEVEL,
            FILTER_SOCKETS,
            FILTER_GEM_LEVEL,
            FILTER_QUALITY,
            FILTER_CATALYST_QUALITY,
            FILTER_REQUIRED_LEVEL,
            FILTER_PHYSICAL_DPS,
            FILTER_ELEMENTAL_DPS,
            FILTER_WAYSTONE_REVIVES,
            FILTER_WAYSTONE_ITEM_RARITY,
            FILTER_WAYSTONE_PACK_SIZE,
            FILTER_WAYSTONE_MONSTER_RARITY,
            FILTER_WAYSTONE_MONSTER_EFFECTIVENESS,
            FILTER_WAYSTONE_DROP_CHANCE,
            PSEUDO_ELEMENTAL_RESISTANCE,
            PSEUDO_CHAOS_RESISTANCE,
            PSEUDO_LIFE,
            PSEUDO_MANA,
            PSEUDO_STRENGTH,
            PSEUDO_DEXTERITY,
            PSEUDO_INTELLIGENCE,
            PSEUDO_ALL_ATTRIBUTES,
            PSEUDO_UNREVEALED_MODIFIERS,
            OFFERS,
            STACK_OF,
            UNIT_PRICE,
            TREND_RISE,
            TREND_FALL,
            TREND_STABLE,
            UNCERTAIN,
            SIMILAR,
            ESTIMATE_UNCERTAIN,
            MEDIAN.plain,
            MEDIAN.of_kind,
            MEDIAN_CHEAPEST.plain,
            MEDIAN_CHEAPEST.of_kind,
            QUALITY_OF_KIND,
            OFFER_QUALITY_KIND,
            PRICES_DOWNLOADING,
            NO_PRICE_FOUND,
            STATUS_TRADE,
            STATUS_TRADE_WAIT,
            OFFERS_SEARCHED,
            OFFERS_COUNT,
            OFFERS_NEXT_SEARCH_TOOLTIP,
            OFFERS_LIMIT,
            OFFERS_REFUSED_HINT,
            OFFERS_UNREADABLE_HINT,
            OFFERS_REPORT,
            OFFERS_REPORT_START,
            THIN_TRADES,
        ])
        .chain(WINDOW_ERROR_IDS)
        .chain(DOWNLOAD_QUOTES)
        .map(str::to_owned)
        .collect();
    for language in Language::ALL {
        assert_eq!(file_ids(language), code_ids, "messages of {language:?}");
    }
}

#[test]
fn the_affix_of_a_lord_names_him_in_each_language() {
    for language in Language::ALL {
        let translator = Translator::new(language);
        for lord in [Lord::Amanamu, Lord::Kurgal, Lord::Ulaman] {
            for affix in [Affix::Prefix, Affix::Suffix] {
                let text = translator.lord_affix(affix, lord);
                assert!(text.contains(lord.name()), "{text} in {language:?}");
            }
        }
    }
    let english = Translator::new(Language::English);
    assert_eq!(
        english.lord_affix(Affix::Prefix, Lord::Amanamu),
        "Amanamu Prefix"
    );
    // The elision of the French: "de Kurgal", "d'Amanamu".
    let french = Translator::new(Language::French);
    assert_eq!(
        french.lord_affix(Affix::Prefix, Lord::Kurgal),
        "Préfixe de Kurgal"
    );
    assert_eq!(
        french.lord_affix(Affix::Suffix, Lord::Amanamu),
        "Suffixe d'Amanamu"
    );
}

/// The messages of a window that can't open.
const WINDOW_ERROR_IDS: [&str; 8] = [
    WINDOW_ERROR_DISPLAY,
    WINDOW_ERROR_DISPLAY_FIX,
    WINDOW_ERROR_OPENGL,
    WINDOW_ERROR_OPENGL_FIX,
    WINDOW_ERROR_OTHER,
    WINDOW_ERROR_OTHER_FIX,
    WINDOW_ERROR_LOG,
    WINDOW_ERROR_DETAILS,
];

#[test]
fn every_message_of_a_window_that_cant_open_formats_without_errors() {
    // The errors fail the `debug_assert` of `format`.
    for language in Language::ALL {
        let translator = Translator::new(language);
        for cause in [Cause::Display, Cause::OpenGl, Cause::Other] {
            let [failed, fix] = translator.window_error(cause);
            assert!(
                failed.contains(crate::APP_NAME),
                "{cause:?} in {language:?}"
            );
            assert!(!fix.is_empty(), "{cause:?} in {language:?}");
        }
        assert!(translator.window_error_log("/log").contains("/log"));
        assert!(translator.window_error_details("failed").contains("failed"));
    }
}

#[test]
fn every_message_formats_without_errors() {
    // The errors fail the `debug_assert` of `format`.
    for language in Language::ALL {
        let translator = Translator::new(language);
        for &message in Message::ALL {
            assert_ne!(translator.text(message), "", "{message:?} in {language:?}");
        }
        assert_ne!(translator.status_bar("A", "B"), "", "{language:?}");
        assert_ne!(translator.league_tooltip("A"), "", "{language:?}");
        assert!(
            translator
                .bug_report_log_session("Session")
                .contains("Session"),
            "{language:?}"
        );
        for age in [0, 60, 3600, 86_400] {
            assert_ne!(translator.age(Duration::from_secs(age)), "", "{language:?}");
        }
        for tag in [NumberTag::Quality, NumberTag::Sockets, NumberTag::ItemLevel] {
            for value in [0, 1, 2, 20] {
                let text = translator.tag(tag, value);
                assert!(text.contains(&value.to_string()), "{text} in {language:?}");
            }
        }
        assert!(translator.stack_tag(7, 20).contains("7/20"), "{language:?}");
        assert!(translator.granted_skill("Skill").contains("Skill"));
        let lines = ["+38 to Armour", "+35 to Evasion"];
        let with = translator.hybrid_with(&lines);
        assert!(
            with.contains(lines[0]) && with.contains(lines[1]),
            "{with} in {language:?}"
        );
        let part = translator.hybrid_part("75–79", "258% increased");
        assert!(
            part.contains("75–79") && part.contains("258% increased"),
            "{part} in {language:?}"
        );
        for count in [0, 1, 4, 23] {
            assert!(translator.offers(count).contains(&count.to_string()));
            assert_ne!(translator.uncertain(count), "");
            let thin = translator.thin_trades(count);
            assert!(
                count < 2 || thin.contains(&count.to_string()),
                "{thin} in {language:?}"
            );
        }
        assert!(translator.stack_of(7).contains('7'));
        for count in [1, 3, 5, 18] {
            assert!(translator.similar(count).contains(&count.to_string()));
        }
        for count in [1, 4] {
            assert_ne!(translator.estimate_uncertain(count), "");
            assert_ne!(translator.estimate_median(count, None), "");
        }
        assert!(translator.estimate_median_cheapest(5, None).contains('5'));
        assert!(translator.show_stats(1).contains('1'));
        assert!(translator.show_stats(6).contains('6'));
        assert!(translator.affix_fill(3, 3).contains('3'));
        assert!(translator.affix_fill(2, 3).contains('1'));
        assert!(translator.active_filters(1).contains('1'));
        assert!(translator.active_filters(3).contains('3'));
        assert!(translator.filter_rolls("-7.5", "60").contains("-7.5"));
        assert!(translator.filter_rolls("-7.5", "60").contains("60"));
        assert!(translator.unit_price("prices").contains("prices"));
        for direction in [Direction::Rise, Direction::Fall, Direction::Stable] {
            assert!(translator.trend(direction, 4, 7).contains('7'));
        }
        assert!(translator.prices_downloading("League").contains("League"));
        assert!(translator.no_price_found("League").contains("League"));
        for requirement in [
            Requirement::Level,
            Requirement::Strength,
            Requirement::Dexterity,
            Requirement::Intelligence,
        ] {
            assert!(translator.requirement(requirement, 65).contains("65"));
        }
        for index in 0..DOWNLOAD_QUOTE_COUNT {
            assert_ne!(translator.download_quote(index), "", "{language:?}");
        }
        for line in [
            translator.status_prices("age"),
            translator.status_prices_offline("age"),
            translator.prices_unreachable("age"),
        ] {
            assert!(line.contains("age"), "{line} in {language:?}");
        }
        assert_ne!(translator.author_line(), "", "{language:?}");
    }
}

#[test]
fn the_open_affixes_agree_in_number_in_french() {
    let french = Translator::new(Language::French);

    assert_eq!(french.affix_fill(2, 3), "2 sur 3 · 1 libre");
    assert_eq!(french.affix_fill(1, 3), "1 sur 3 · 2 libres");
    assert_eq!(french.affix_fill(3, 3), "3 sur 3");
}

#[test]
fn arguments_are_inserted_without_isolation_marks() {
    assert_eq!(
        Translator::new(Language::English).status_bar("A", "B"),
        "A · B"
    );
}

#[test]
fn the_author_line_ends_with_the_site_in_every_language() {
    // The status bar draws the site as a link, cut from the end of the line.
    for language in Language::ALL {
        let line = Translator::new(language).author_line();

        let running = format!("{} {} · ", crate::APP_NAME, crate::VERSION);
        assert!(line.starts_with(&running), "{line}");
        let site = format!("{} · {}", crate::AUTHOR, crate::AUTHOR_SITE_LABEL);
        assert!(line.ends_with(&site), "{line}");
    }
}

fn around(before: &str, after: &str) -> Around {
    Around {
        before: before.to_owned(),
        after: after.to_owned(),
    }
}

#[test]
fn a_sentence_is_cut_around_its_argument() {
    let english = Translator::new(Language::English);

    assert_eq!(english.new_league(), around("", " has started."));
    assert_eq!(
        english.update_available("App"),
        around("App ", " is available.")
    );
    assert_eq!(
        english.download_title(),
        around("Downloading the prices of ", "")
    );
    assert_eq!(english.requires(), around("Requires ", ""));
    // Cut where the argument goes, not where its text is first found in the sentence.
    let join = english.bug_report_join_item();
    assert_eq!(join, around("Join the item shown: ", ""));
    assert_eq!(join.with("a"), "Join the item shown: a");
}

#[test]
fn every_sentence_cut_around_an_argument_holds_it_in_every_language() {
    // `Translator::around` asserts that the translation holds the argument.
    for language in Language::ALL {
        let translator = Translator::new(language);
        for sentence in [
            translator.new_league(),
            translator.update_available("App"),
            translator.download_title(),
            translator.bug_report_join_item(),
            translator.requires(),
        ] {
            assert_ne!(sentence.with(""), "", "{language:?}");
        }
    }
}

#[test]
fn the_fill_of_the_affixes_tells_the_open_ones() {
    let english = Translator::new(Language::English);

    assert_eq!(english.affix_fill(3, 3), "3 of 3");
    assert_eq!(english.affix_fill(2, 3), "2 of 3 · 1 open");
}

/// The marks of `line`, a line of a translation file, that don't follow a non-breaking
/// space (ordinary or narrow): the code between braces and the address of a link aside.
fn marks_without_their_space(line: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut depth = 0_u32;
    let mut previous = None;
    for (index, character) in line.char_indices() {
        match character {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ':' | ';' | '?' | '!' if depth == 0 => {
                let address = line[index..].starts_with("://");
                if !address && !matches!(previous, Some('\u{a0}' | '\u{202f}')) {
                    found.push(format!("{character} in \"{line}\""));
                }
            }
            _ => {}
        }
        previous = Some(character);
    }
    found
}

#[test]
fn a_french_punctuation_mark_follows_a_non_breaking_space() {
    // egui breaks a line at an ordinary space only: a wrapped text never starts a line with
    // ":". The interface review of 2026-11-10: a mark right after its word passed too.
    // A quote in English keeps the typography of English.
    const ENGLISH_QUOTES: [&str; 1] = ["still available?"];
    let mut found = Vec::new();
    for line in messages(Language::French)
        .lines()
        .filter(|line| !line.trim_start().starts_with('#'))
        .filter(|line| !ENGLISH_QUOTES.iter().any(|quote| line.contains(quote)))
    {
        found.extend(marks_without_their_space(line));
    }

    assert!(found.is_empty(), "{found:#?}");
}

#[test]
fn a_mark_right_after_its_word_or_after_an_ordinary_space_is_found() {
    let none = Vec::<String>::new();
    assert_eq!(marks_without_their_space("base = Base: { $base }").len(), 1);
    assert_eq!(
        marks_without_their_space("base = Base : { $base }").len(),
        1
    );
    assert_eq!(
        marks_without_their_space("base = Base\u{a0}: { $base }"),
        none
    );
    assert_eq!(marks_without_their_space("base = Prêt\u{202f}?"), none);
    // The code of Fluent and the address of a link.
    assert_eq!(
        marks_without_their_space("a = { $count -> [one] b; *[other] c! }"),
        none
    );
    assert_eq!(
        marks_without_their_space("site = https://example.org"),
        none
    );
}

#[test]
fn no_offer_has_its_own_warning_in_every_language() {
    // In French, the plural category "one" covers 0 too.
    let french = Translator::new(Language::French);
    assert!(french.uncertain(0).starts_with("Aucune offre"));
    assert!(french.uncertain(1).starts_with("Une seule offre"));
    assert!(french.thin_trades(0).starts_with("Aucun échange"));
    assert!(french.thin_trades(1).starts_with("Un seul échange"));
    let english = Translator::new(Language::English);
    assert!(english.uncertain(0).starts_with("No offer"));
    assert!(english.thin_trades(0).starts_with("No trade"));
}

#[test]
fn an_estimate_from_one_offer_is_its_price() {
    for language in Language::ALL {
        let translator = Translator::new(language);
        assert!(
            !translator.estimate_median(1, None).contains('1'),
            "{language:?}"
        );
        assert!(translator.estimate_median(3, None).contains('3'));
    }
    assert_eq!(
        Translator::new(Language::English).estimate_uncertain(3),
        "Only 3 similar offers: the estimate is uncertain"
    );
}

#[test]
fn an_age_is_rounded_down_to_its_largest_unit() {
    let english = Translator::new(Language::English);
    let age = |seconds| english.age(Duration::from_secs(seconds));

    assert_eq!(age(59), "just now");
    assert_eq!(age(12 * 60 + 59), "12min ago");
    assert_eq!(age(3 * 3600 + 59 * 60), "3h ago");
    assert_eq!(age(2 * 86_400 + 3600), "2d ago");
    // Glued to the unit in French too.
    let french = Translator::new(Language::French);
    assert_eq!(french.age(Duration::from_secs(59)), "à l'instant");
    assert_eq!(french.age(Duration::from_mins(12)), "il y a 12min");
}

#[test]
fn a_wait_is_rounded_up_so_that_it_never_ends_too_soon() {
    let english = Translator::new(Language::English);
    assert_eq!(english.wait(Duration::from_millis(8_200)), "9s");
    assert_eq!(english.wait(Duration::from_secs(8)), "8s");
    assert_eq!(english.wait(Duration::from_secs(59)), "59s");
    assert_eq!(english.wait(Duration::from_secs(60)), "1min");
    assert_eq!(english.wait(Duration::from_secs(181)), "4min");
    let french = Translator::new(Language::French);
    assert_eq!(
        french.offers_limit(Duration::from_secs(240)),
        "Limite du site d'échange\u{a0}: prochaine recherche dans 4min."
    );
}

#[test]
fn the_quality_of_a_catalyst_is_short_with_its_kind() {
    let english = Translator::new(Language::English);
    let french = Translator::new(Language::French);
    let known = |catalyst| QualityKind {
        catalyst: Some(catalyst),
        words: String::new(),
    };
    let mana = known(Catalyst::Mana);
    let attack = known(Catalyst::Attack);
    let unknown = QualityKind {
        catalyst: None,
        words: "Speedy Modifiers".to_owned(),
    };

    assert_eq!(
        english.filter_catalyst_quality(&mana, "40"),
        "Mana quality: 40%"
    );
    assert_eq!(
        french.filter_catalyst_quality(&mana, "40"),
        "Qualité de mana\u{a0}: 40%"
    );
    assert_eq!(
        french.filter_catalyst_quality(&attack, "20"),
        "Qualité d'attaque\u{a0}: 20%"
    );
    // A kind the application doesn't know keeps the words of the game.
    assert_eq!(
        english.filter_catalyst_quality(&unknown, "20"),
        "Quality (Speedy Modifiers): 20%"
    );
}

#[test]
fn the_note_of_an_estimate_names_the_kind_of_quality_searched() {
    let english = Translator::new(Language::English);
    let french = Translator::new(Language::French);

    assert_eq!(
        english.estimate_median_cheapest(5, Some(Catalyst::Mana)),
        "Median of the 5 cheapest similar offers of mana quality."
    );
    assert_eq!(
        english.estimate_median_cheapest(5, None),
        "Median of the 5 cheapest similar offers."
    );
    assert_eq!(
        english.estimate_median(1, Some(Catalyst::Attack)),
        "Price of the only similar offer of attack quality."
    );
    assert_eq!(
        french.estimate_median_cheapest(5, Some(Catalyst::Mana)),
        "Médiane des 5 offres similaires les moins chères de qualité de mana."
    );
    assert_eq!(
        french.estimate_median(3, Some(Catalyst::Attack)),
        "Médiane des 3 offres similaires de qualité d'attaque."
    );
    assert_eq!(
        french.offer_quality_kind(Catalyst::Caster),
        "Qualité de sorts"
    );
}

#[test]
fn every_kind_of_catalyst_has_its_text_in_every_language() {
    for language in [Language::English, Language::French] {
        let translator = Translator::new(language);
        for catalyst in Catalyst::ALL {
            let kind = QualityKind {
                catalyst: Some(catalyst),
                words: "the words of the game".to_owned(),
            };
            let text = translator.filter_catalyst_quality(&kind, "40");
            // Not the fallback with the words of the game.
            assert!(
                !text.contains("the words") && text.contains("40%"),
                "{catalyst:?} in {language:?}: {text}"
            );
            // Not the default variants.
            let note = translator.estimate_median(3, Some(catalyst));
            let word = translator.offer_quality_kind(catalyst);
            assert!(
                !["Other quality", "Autre qualité"].contains(&word.as_str())
                    && !note.ends_with(" of quality.")
                    && !note.ends_with(" de qualité."),
                "{catalyst:?} in {language:?}: {word}, {note}"
            );
        }
    }
}

#[test]
fn the_level_and_the_sockets_of_a_gem_are_named_in_each_language() {
    let english = Translator::new(Language::English);
    let french = Translator::new(Language::French);
    assert_eq!(english.filter_property(Property::GemLevel, "2"), "Level: 2");
    assert_eq!(
        french.filter_property(Property::GemLevel, "2"),
        "Niveau\u{a0}: 2"
    );
    // The same words as the sockets of another item.
    assert_eq!(
        english.filter_property(Property::GemSockets, "5"),
        "Sockets: 5"
    );
    assert_eq!(
        french.filter_property(Property::GemSockets, "5"),
        "Châsses\u{a0}: 5"
    );
    assert_eq!(english.gem_level(21), "Level\u{a0}21");
    assert_eq!(french.gem_level(21), "Niveau\u{a0}21");
}

#[test]
fn the_hint_of_a_failed_search_holds_its_link_in_each_language() {
    for language in Language::ALL {
        let translator = Translator::new(language);
        for (hint, words) in [
            (translator.offers_refused_hint(), OFFERS_REPORT),
            (translator.offers_unreadable_hint(), OFFERS_REPORT_START),
        ] {
            let words = translator.format(words, None);
            assert_eq!(
                hint.text.get(hint.link.clone()),
                Some(words.as_str()),
                "{hint:?} in {language:?}"
            );
        }
    }
    let english = Translator::new(Language::English).offers_unreadable_hint();
    assert_eq!(english.text.get(english.link), Some("Report it"));
}
