//! Markdown in, prose out: the extraction stage the rest of the pipeline
//! reads from.
//!
//! The fog formula only ever sees prose, so the never-scored constructs
//! (headings, code blocks, tables, image alt text, raw HTML, task-list
//! markers) are removed here. Inline code spans and bare URLs/autolinks
//! become [`ProseEvent::Placeholder`] so a sentence keeps its length.
//! Structure the later stages need (list-item edges, soft/hard breaks,
//! paragraph ends, source byte spans) is carried as events rather than
//! flattened away.
//!
//! Text is normally carried as the parser decodes it (entity references
//! resolved). The exception is a run whose source holds a bare-URL
//! candidate: markdown specials inside a URL split it across parser events,
//! so those runs are read straight from the source, where the URL's true
//! extent is visible.

use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag, TagEnd};

/// One item in the prose stream [`extract`] produces.
///
/// `Text` and `Placeholder` carry their byte extent in the source, so
/// later stages can turn them into line numbers and show a placeholder's
/// construct as written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProseEvent {
    /// A run of prose text.
    Text {
        /// The text, entity-decoded except in URL-bearing runs (see the
        /// module docs).
        text: String,
        /// The run's byte extent in the markdown source.
        span: Range<usize>,
    },
    /// One placeholder word standing in for an inline code span or a bare
    /// URL/autolink: exactly one word of one syllable, never complex.
    Placeholder {
        /// The replaced construct's byte extent in the markdown source.
        span: Range<usize>,
    },
    /// A soft break; becomes a space before segmentation.
    SoftBreak,
    /// A sentence boundary: a hard break, a paragraph end, a list item's
    /// start or end, or a removed block-level construct (heading, code
    /// block, table, HTML block) where it sits. Never emitted first or
    /// twice in a row.
    Boundary,
}

/// Extracts the prose stream from markdown source.
///
/// ```
/// use gunfog::prose::{ProseEvent, extract};
///
/// assert_eq!(
///     extract("Run `cargo test` now."),
///     vec![
///         ProseEvent::Text { text: "Run ".into(), span: 0..4 },
///         ProseEvent::Placeholder { span: 4..16 },
///         ProseEvent::Text { text: " now.".into(), span: 16..21 },
///         ProseEvent::Boundary,
///     ],
/// );
/// ```
///
/// Author: Claude Fable 5
pub fn extract(markdown: &str) -> Vec<ProseEvent> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_STRIKETHROUGH;
    let parsed: Vec<(Event, Range<usize>)> = Parser::new_ext(markdown, options)
        .into_offset_iter()
        .collect();
    let bounds = link_text_bounds(&parsed);
    let mut events = Vec::new();
    // Depth of nesting inside never-scored constructs; text only flows at 0.
    let mut skip_depth = 0usize;
    // One entry per open link: `None` for an autolink, whose text (the URL
    // itself) is already replaced by its placeholder, or the link text's
    // extent, which bounds a bare-URL scan so it cannot escape into the
    // link markup and destination. Autolinks nest inside link text, so this
    // is a stack, not a flag.
    let mut links: Vec<Option<usize>> = Vec::new();
    // A bare URL can span several parser events when markdown specials
    // inside it split the run. Everything before this source position is
    // already covered by the URL's placeholder.
    let mut consumed_until = 0usize;
    for (index, (event, range)) in parsed.into_iter().enumerate() {
        match event {
            // A removed block-level construct is also a sentence boundary:
            // a tight list item emits no paragraph events, so without one
            // the text either side of the construct would fuse. Image is
            // the one inline construct in this list and emits none; the
            // prose either side of an image is a single sentence.
            Event::Start(
                tag @ (Tag::Heading { .. }
                | Tag::CodeBlock(_)
                | Tag::Table(_)
                | Tag::Image { .. }
                | Tag::HtmlBlock),
            ) => {
                if !matches!(tag, Tag::Image { .. }) {
                    push_boundary(&mut events);
                }
                skip_depth += 1;
            }
            Event::End(
                TagEnd::Heading(_)
                | TagEnd::CodeBlock
                | TagEnd::Table
                | TagEnd::Image
                | TagEnd::HtmlBlock,
            ) => skip_depth -= 1,
            _ if skip_depth > 0 => {}
            Event::Start(Tag::Link { link_type, .. }) => match link_type {
                LinkType::Autolink | LinkType::Email => {
                    // A bare URL always stops at the `<` that opens an
                    // autolink, so this guard is defensive only.
                    if range.start >= consumed_until {
                        events.push(ProseEvent::Placeholder { span: range });
                    }
                    links.push(None);
                }
                _ => links.push(Some(bounds[index])),
            },
            Event::End(TagEnd::Link) => {
                links.pop();
            }
            Event::Text(text) => match links.last() {
                Some(None) => {}
                Some(Some(bound)) => emit_text(
                    markdown,
                    &text,
                    range,
                    *bound,
                    &mut consumed_until,
                    &mut events,
                ),
                None => emit_text(
                    markdown,
                    &text,
                    range,
                    markdown.len(),
                    &mut consumed_until,
                    &mut events,
                ),
            },
            Event::Code(_) => {
                // A code span can straddle the end of a consumed URL; the
                // uncovered remainder is still one placeholder word.
                if range.end > consumed_until {
                    events.push(ProseEvent::Placeholder {
                        span: range.start.max(consumed_until)..range.end,
                    });
                }
            }
            Event::Start(Tag::Item) | Event::End(TagEnd::Paragraph | TagEnd::Item) => {
                push_boundary(&mut events)
            }
            Event::SoftBreak => events.push(ProseEvent::SoftBreak),
            Event::HardBreak => push_boundary(&mut events),
            _ => {}
        }
    }
    events
}

/// For each regular link's `Start` event, indexed by event position, the
/// extent of its link text: the maximum `range.end` over the events inside
/// the link. Every inner event lies within the link text (the destination
/// is never an event), so the extent ends at or before the closing `]` and
/// a bare-URL scan bounded by it cannot escape into the link markup.
///
/// Author: Claude Fable 5
fn link_text_bounds(parsed: &[(Event, Range<usize>)]) -> Vec<usize> {
    let mut bounds = vec![0; parsed.len()];
    // One entry per open link: whether it is a regular link, its event
    // index, and the running extent (falling back to the text's start for
    // empty link text, where no scan can happen anyway).
    let mut stack: Vec<(bool, usize, usize)> = Vec::new();
    for (index, (event, range)) in parsed.iter().enumerate() {
        match event {
            Event::Start(Tag::Link { link_type, .. }) => {
                // The nested link sits inside the outer link's text.
                if let Some(outer) = stack.last_mut() {
                    outer.2 = outer.2.max(range.end);
                }
                let regular = !matches!(link_type, LinkType::Autolink | LinkType::Email);
                stack.push((regular, index, range.start + 1));
            }
            Event::End(TagEnd::Link) => {
                if let Some((regular, start, extent)) = stack.pop()
                    && regular
                {
                    bounds[start] = extent;
                }
            }
            _ => {
                if let Some(top) = stack.last_mut() {
                    top.2 = top.2.max(range.end);
                }
            }
        }
    }
    bounds
}

/// Emits one text run: entity-decoded when clean, straight from the source
/// when the run holds a bare-URL candidate or a URL from an earlier event
/// reaches into it. `token_limit` caps how far a URL may extend: the link
/// text's end inside a link, the source's end otherwise.
///
/// Author: Claude Fable 5
fn emit_text(
    source: &str,
    decoded: &str,
    range: Range<usize>,
    token_limit: usize,
    consumed_until: &mut usize,
    events: &mut Vec<ProseEvent>,
) {
    if range.end <= *consumed_until {
        return;
    }
    let start = range.start.max(*consumed_until);
    if start == range.start && find_http_ci(&source[range.start..range.end]).is_none() {
        // The common case: no URL near this run.
        events.push(ProseEvent::Text {
            text: decoded.to_string(),
            span: range,
        });
        return;
    }
    scan_bare_urls(
        source,
        start..range.end,
        token_limit,
        consumed_until,
        events,
    );
}

/// Scans a source region for bare URLs, emitting the text between them as
/// written and one placeholder per URL.
///
/// A bare URL starts with `http://` or `https://` (any letter case) at the
/// region's start or after a non-alphanumeric character, and runs to the
/// next whitespace or `<` before `token_limit`. The limit is the source's
/// end normally, letting a URL run past the region when markdown specials
/// split it into several events, and the link text's end inside a link;
/// [`trim_url`] then gives trailing punctuation back to the sentence. Other
/// schemes and scheme-less hosts (`www.…`) stay text.
///
/// Author: Claude Fable 5
fn scan_bare_urls(
    source: &str,
    region: Range<usize>,
    token_limit: usize,
    consumed_until: &mut usize,
    events: &mut Vec<ProseEvent>,
) {
    let mut plain_start = region.start;
    let mut scan = region.start;
    while scan < region.end {
        let Some(rel) = find_http_ci(&source[scan..region.end]) else {
            break;
        };
        let candidate = scan + rel;
        let rest = &source[candidate..];
        let scheme_len = if starts_with_ci(rest, "https://") {
            8
        } else if starts_with_ci(rest, "http://") {
            7
        } else {
            scan = candidate + 4;
            continue;
        };
        let word_boundary = candidate == 0
            || source[..candidate]
                .chars()
                .next_back()
                .is_some_and(|c| !c.is_alphanumeric());
        if !word_boundary {
            scan = candidate + scheme_len;
            continue;
        }
        let token_end = source[candidate..token_limit]
            .find(|c: char| c.is_whitespace() || c == '<')
            .map_or(token_limit, |i| candidate + i);
        let url_end = candidate + trim_url(&source[candidate..token_end]).len();
        if url_end == candidate + scheme_len {
            scan = candidate + scheme_len;
            continue;
        }
        if candidate > plain_start {
            events.push(ProseEvent::Text {
                text: source[plain_start..candidate].to_string(),
                span: plain_start..candidate,
            });
        }
        events.push(ProseEvent::Placeholder {
            span: candidate..url_end,
        });
        *consumed_until = url_end;
        plain_start = url_end;
        scan = url_end;
    }
    if plain_start < region.end {
        events.push(ProseEvent::Text {
            text: source[plain_start..region.end].to_string(),
            span: plain_start..region.end,
        });
    }
}

/// Case-insensitive search for the ASCII needle `http`. A match can only
/// start on a character boundary, because UTF-8 continuation bytes never
/// equal an ASCII letter.
///
/// Author: Claude Fable 5
fn find_http_ci(s: &str) -> Option<usize> {
    s.as_bytes()
        .windows(4)
        .position(|w| w.eq_ignore_ascii_case(b"http"))
}

/// Case-insensitive ASCII prefix test.
///
/// Author: Claude Fable 5
fn starts_with_ci(s: &str, prefix: &str) -> bool {
    s.len() >= prefix.len() && s.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
}

/// Gives back the punctuation a sentence hangs off a bare URL's end:
/// GFM's extended-autolink trimming (its punctuation and quote set, a `)`
/// only while the token holds more closing than opening parens, and a `;`
/// closing an entity reference takes the whole entity with it), plus `…`.
/// A `;` that closes no entity stays part of the URL, a deliberate
/// departure from GFM, which trims a lone semicolon: keeping it avoids
/// leaving a trailing half-entity behind.
///
/// The paren counts are taken once and kept current as parens are trimmed
/// (nothing else in the cut set contains one), so a paren flood trims in
/// linear time.
///
/// Author: Claude Fable 5
fn trim_url(token: &str) -> &str {
    let opens = token.matches('(').count();
    let mut closes = token.matches(')').count();
    let mut url = token;
    while let Some(last) = url.chars().next_back() {
        let cut_len = match last {
            ')' if closes > opens => {
                closes -= 1;
                1
            }
            ';' => trailing_entity_len(url).unwrap_or(0),
            '.' | ',' | ':' | '!' | '?' | '*' | '_' | '~' | '"' | '\'' => 1,
            '…' => last.len_utf8(),
            _ => 0,
        };
        if cut_len == 0 {
            break;
        }
        url = &url[..url.len() - cut_len];
    }
    url
}

/// Byte length of the entity reference (`&`, one or more ASCII
/// alphanumerics, `;`) ending `url`, if one does. The name class is wider
/// than GFM's alpha-only one on purpose: digit-bearing entities like
/// `&frac12;` come back whole.
///
/// Author: Claude Fable 5
fn trailing_entity_len(url: &str) -> Option<usize> {
    let body = url.strip_suffix(';')?;
    let name_len = body
        .chars()
        .rev()
        .take_while(char::is_ascii_alphanumeric)
        .count();
    if name_len == 0 {
        return None;
    }
    body[..body.len() - name_len]
        .ends_with('&')
        .then_some(name_len + 2)
}

/// Pushes a sentence boundary, collapsing adjacent ones and dropping a
/// leading one.
///
/// Author: Claude Fable 5
fn push_boundary(events: &mut Vec<ProseEvent>) {
    match events.last() {
        None | Some(ProseEvent::Boundary) => {}
        Some(_) => events.push(ProseEvent::Boundary),
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;

    use super::{ProseEvent, extract};

    /// Shorthand for a [`ProseEvent::Text`] whose text matches its source
    /// bytes, so the span end is the start plus the byte length.
    ///
    /// Author: Claude Fable 5
    fn text(text: &str, start: usize) -> ProseEvent {
        ProseEvent::Text {
            text: text.to_string(),
            span: start..start + text.len(),
        }
    }

    /// Author: Claude Fable 5
    fn placeholder(span: Range<usize>) -> ProseEvent {
        ProseEvent::Placeholder { span }
    }

    /// Author: Claude Fable 5
    #[test]
    fn plain_text_passes_through_unchanged() {
        assert_eq!(
            extract("The fog rolled in."),
            vec![text("The fog rolled in.", 0), ProseEvent::Boundary],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn headings_are_removed() {
        let md = "# ATX heading\n\nBody one.\n\nSetext heading\n==============\n\nBody two.";
        assert_eq!(
            extract(md),
            vec![
                text("Body one.", 15),
                ProseEvent::Boundary,
                text("Body two.", 57),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn fenced_code_blocks_are_removed() {
        let md = "Before.\n\n```rust\nlet x = 1;\n```\n\nAfter.";
        assert_eq!(
            extract(md),
            vec![
                text("Before.", 0),
                ProseEvent::Boundary,
                text("After.", 33),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn indented_code_blocks_are_removed() {
        let md = "Before.\n\n    let x = 1;\n\nAfter.";
        assert_eq!(
            extract(md),
            vec![
                text("Before.", 0),
                ProseEvent::Boundary,
                text("After.", 25),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn tables_are_removed() {
        let md = "Before.\n\n| a | b |\n|---|---|\n| c | d |\n\nAfter.";
        assert_eq!(
            extract(md),
            vec![
                text("Before.", 0),
                ProseEvent::Boundary,
                text("After.", 40),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn image_alt_text_is_removed() {
        assert_eq!(
            extract("See ![alt text](img.png) for detail."),
            vec![
                text("See ", 0),
                text(" for detail.", 24),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn html_blocks_are_removed() {
        let md = "<div>\nnot prose\n</div>\n\nAfter.";
        assert_eq!(extract(md), vec![text("After.", 24), ProseEvent::Boundary]);
    }

    /// Author: Claude Fable 5
    #[test]
    fn inline_html_is_removed_but_its_content_kept() {
        assert_eq!(
            extract("Some <b>bold</b> words."),
            vec![
                text("Some ", 0),
                text("bold", 8),
                text(" words.", 16),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn html_comments_are_removed() {
        assert_eq!(
            extract("Line one <!-- aside --> continues."),
            vec![
                text("Line one ", 0),
                text(" continues.", 23),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn block_html_comments_are_removed() {
        assert_eq!(
            extract("<!-- note -->\n\nAfter."),
            vec![text("After.", 15), ProseEvent::Boundary],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn blockquote_content_is_scored() {
        assert_eq!(
            extract("> Quoted prose here."),
            vec![text("Quoted prose here.", 2), ProseEvent::Boundary],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn footnote_definitions_are_scored_and_references_dropped() {
        let md = "Cited claim.[^1]\n\n[^1]: The note body.";
        assert_eq!(
            extract(md),
            vec![
                text("Cited claim.", 0),
                ProseEvent::Boundary,
                text("The note body.", 24),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn link_text_is_kept_and_url_dropped() {
        assert_eq!(
            extract("See [the spec](https://example.com) now."),
            vec![
                text("See ", 0),
                text("the spec", 5),
                text(" now.", 35),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn reference_and_shortcut_link_text_is_kept() {
        let md = "Read [the guide][g] and [manual] today.\n\n\
                  [g]: https://example.com/g\n[manual]: https://example.com/m";
        assert_eq!(
            extract(md),
            vec![
                text("Read ", 0),
                text("the guide", 6),
                text(" and ", 19),
                text("manual", 25),
                text(" today.", 32),
                ProseEvent::Boundary,
            ],
        );
    }

    /// The bound comes from the link's inner events, so it holds for a
    /// reference link's `]` just as it does for an inline link's.
    ///
    /// Author: Claude Opus 5
    #[test]
    fn bare_url_as_reference_link_text_keeps_its_own_span() {
        let md = "[https://x.com][g] end.\n\n[g]: https://y.com";
        assert_eq!(
            extract(md),
            vec![placeholder(1..14), text(" end.", 18), ProseEvent::Boundary],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn image_alt_inside_link_text_is_removed() {
        assert_eq!(
            extract("See [![logo](l.png) the docs](https://x.example) now."),
            vec![
                text("See ", 0),
                text(" the docs", 19),
                text(" now.", 48),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn emphasis_and_strong_are_transparent() {
        assert_eq!(
            extract("Some *emphasised* and **strong** words."),
            vec![
                text("Some ", 0),
                text("emphasised", 6),
                text(" and ", 17),
                text("strong", 24),
                text(" words.", 32),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn strikethrough_is_transparent() {
        assert_eq!(
            extract("Some ~~struck~~ words."),
            vec![
                text("Some ", 0),
                text("struck", 7),
                text(" words.", 15),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn task_list_markers_are_removed() {
        assert_eq!(
            extract("- [x] done item\n- [ ] open item"),
            vec![
                text("done item", 6),
                ProseEvent::Boundary,
                text("open item", 22),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn inline_code_span_becomes_one_placeholder() {
        assert_eq!(
            extract("Run `cargo test` before pushing."),
            vec![
                text("Run ", 0),
                placeholder(4..16),
                text(" before pushing.", 16),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn autolink_becomes_one_placeholder() {
        assert_eq!(
            extract("Docs live at <https://example.com> now."),
            vec![
                text("Docs live at ", 0),
                placeholder(13..34),
                text(" now.", 34),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn email_autolink_becomes_one_placeholder() {
        assert_eq!(
            extract("Mail <user@example.com> today."),
            vec![
                text("Mail ", 0),
                placeholder(5..23),
                text(" today.", 23),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn bare_url_becomes_one_placeholder() {
        assert_eq!(
            extract("Docs live at https://example.com now."),
            vec![
                text("Docs live at ", 0),
                placeholder(13..32),
                text(" now.", 32),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn bare_url_leaves_trailing_punctuation_as_text() {
        assert_eq!(
            extract("Docs live at https://example.com."),
            vec![
                text("Docs live at ", 0),
                placeholder(13..32),
                text(".", 32),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn bare_url_after_punctuation_is_still_replaced() {
        assert_eq!(
            extract("A quoted \"https://x.example.com\" url."),
            vec![
                text("A quoted \"", 0),
                placeholder(10..31),
                text("\" url.", 31),
                ProseEvent::Boundary,
            ],
        );
    }

    /// The Wikipedia shape: the underscore splits the run into three parser
    /// events, and the parens must survive trimming because they balance.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn bare_url_split_by_markdown_specials_is_one_placeholder() {
        assert_eq!(
            extract("See https://en.wikipedia.org/wiki/Foo_(bar) here now."),
            vec![
                text("See ", 0),
                placeholder(4..43),
                text(" here now.", 43),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn bare_url_with_entity_reference_is_one_placeholder() {
        assert_eq!(
            extract("See https://x.example.com/?a=1&amp;b=2 here."),
            vec![
                text("See ", 0),
                placeholder(4..38),
                text(" here.", 38),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn bare_url_split_by_emphasis_is_one_placeholder() {
        assert_eq!(
            extract("See https://ex.com/a*b*c here."),
            vec![
                text("See ", 0),
                placeholder(4..24),
                text(" here.", 24),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn bare_url_scheme_matches_any_case() {
        assert_eq!(
            extract("See HTTPS://EXAMPLE.COM now."),
            vec![
                text("See ", 0),
                placeholder(4..23),
                text(" now.", 23),
                ProseEvent::Boundary,
            ],
        );
    }

    /// A quadratic trim made 64,000 trailing parens take seconds in
    /// release builds; this must stay linear.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn unmatched_paren_flood_trims_in_linear_time() {
        let parens = ")".repeat(64_000);
        let md = format!("See https://x.com/{parens} end.");
        assert_eq!(
            extract(&md),
            vec![
                text("See ", 0),
                placeholder(4..18),
                text(&format!("{parens} end."), 18),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn code_span_straddling_a_url_end_still_becomes_a_placeholder() {
        assert_eq!(
            extract("See https://ex.com/a`b c`d here."),
            vec![
                text("See ", 0),
                placeholder(4..22),
                placeholder(22..25),
                text("d here.", 25),
                ProseEvent::Boundary,
            ],
        );
    }

    /// GFM: a trailing `;` closing an entity reference excludes the whole
    /// entity from the URL.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn bare_url_gives_back_a_trailing_entity_reference() {
        assert_eq!(
            extract("Read https://x.com/doc&nbsp; now."),
            vec![
                text("Read ", 0),
                placeholder(5..22),
                ProseEvent::Text {
                    text: "\u{a0}".into(),
                    span: 22..28,
                },
                text(" now.", 28),
                ProseEvent::Boundary,
            ],
        );
    }

    /// GFM: a `;` that closes no entity stays part of the URL, whole.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn bare_url_keeps_a_non_entity_semicolon() {
        assert_eq!(
            extract("See https://x.com/a&amp;amp; here."),
            vec![
                text("See ", 0),
                placeholder(4..28),
                text(" here.", 28),
                ProseEvent::Boundary,
            ],
        );
    }

    /// A `]` is an ordinary URL character: IPv6 hosts and array-style
    /// query parameters must come back whole.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn bare_url_keeps_brackets_in_host_and_query() {
        assert_eq!(
            extract("Grab http://[2001:db8::1]/index.html now."),
            vec![
                text("Grab ", 0),
                placeholder(5..36),
                text(" now.", 36),
                ProseEvent::Boundary,
            ],
        );
        assert_eq!(
            extract("Try https://api.x.com/v1?filter[status]=open&page=2 now."),
            vec![
                text("Try ", 0),
                placeholder(4..51),
                text(" now.", 51),
                ProseEvent::Boundary,
            ],
        );
    }

    /// GFM: `<` ends a bare URL.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn bare_url_ends_at_an_angle_bracket() {
        assert_eq!(
            extract("See https://x.com<b>bold</b> now."),
            vec![
                text("See ", 0),
                placeholder(4..17),
                text("bold", 20),
                text(" now.", 28),
                ProseEvent::Boundary,
            ],
        );
    }

    /// An autolink nested in link text must not clear the outer link's
    /// bound: the second URL stops at the link text's end and the word
    /// after the link survives.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn autolink_nested_in_link_text_keeps_the_outer_bound() {
        assert_eq!(
            extract("[a <http://x> http://t](http://d)word more."),
            vec![
                text("a ", 1),
                placeholder(3..13),
                text(" ", 13),
                placeholder(14..22),
                text("word more.", 33),
                ProseEvent::Boundary,
            ],
        );
    }

    /// A link-text URL split by markdown specials is still one placeholder
    /// bounded to the link text: the Wikipedia link shape, linked.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn split_url_as_link_text_is_one_placeholder() {
        let md =
            "[https://en.wikipedia.org/wiki/Interoperability_(computing)](https://x.com) tail.";
        assert_eq!(
            extract(md),
            vec![placeholder(1..59), text(" tail.", 75), ProseEvent::Boundary],
        );
        assert_eq!(
            extract("[https://*a*.com/x](https://y.com) end."),
            vec![placeholder(1..18), text(" end.", 34), ProseEvent::Boundary],
        );
    }

    /// Inside link text the URL scan is bounded to the link text, so link
    /// text that is itself a URL cannot swallow the link markup and
    /// destination into its span.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn bare_url_as_link_text_keeps_its_own_span() {
        assert_eq!(
            extract("See [https://x.com](https://y.com) end."),
            vec![
                text("See ", 0),
                placeholder(5..18),
                text(" end.", 34),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn bare_url_gives_back_trailing_ellipsis() {
        assert_eq!(
            extract("See https://x.com… now."),
            vec![
                text("See ", 0),
                placeholder(4..17),
                text("… now.", 17),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn bare_url_in_parens_gives_back_the_paren() {
        assert_eq!(
            extract("(see https://x.com) now."),
            vec![
                text("(see ", 0),
                placeholder(5..18),
                text(") now.", 18),
                ProseEvent::Boundary,
            ],
        );
    }

    /// The emphasis delimiters flank the whole URL, so trimming hands the
    /// closing `*` back and the delimiter pair vanishes with the markup.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn emphasised_bare_url_is_one_placeholder() {
        assert_eq!(
            extract("*https://x.com* here."),
            vec![placeholder(1..14), text(" here.", 15), ProseEvent::Boundary],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn mid_word_scheme_is_not_a_bare_url() {
        assert_eq!(
            extract("The string ishttps://nope stays text."),
            vec![
                text("The string ishttps://nope stays text.", 0),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn schemeless_host_stays_text() {
        assert_eq!(
            extract("Visit www.example.com now."),
            vec![text("Visit www.example.com now.", 0), ProseEvent::Boundary],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn soft_break_is_kept_for_segmentation() {
        assert_eq!(
            extract("line one\nline two"),
            vec![
                text("line one", 0),
                ProseEvent::SoftBreak,
                text("line two", 9),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn hard_break_is_a_sentence_boundary() {
        assert_eq!(
            extract("line one  \nline two"),
            vec![
                text("line one", 0),
                ProseEvent::Boundary,
                text("line two", 11),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn paragraph_end_is_a_sentence_boundary() {
        assert_eq!(
            extract("First paragraph.\n\nSecond paragraph."),
            vec![
                text("First paragraph.", 0),
                ProseEvent::Boundary,
                text("Second paragraph.", 18),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn list_item_edges_are_sentence_boundaries() {
        assert_eq!(
            extract("Intro line:\n\n- first item\n- second item"),
            vec![
                text("Intro line:", 0),
                ProseEvent::Boundary,
                text("first item", 15),
                ProseEvent::Boundary,
                text("second item", 28),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn nested_list_items_follow_the_same_rule() {
        assert_eq!(
            extract("- outer item\n  - inner item"),
            vec![
                text("outer item", 2),
                ProseEvent::Boundary,
                text("inner item", 17),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn one_bullet_can_hold_several_sentences() {
        assert_eq!(
            extract("- One here. Two here."),
            vec![text("One here. Two here.", 2), ProseEvent::Boundary],
        );
    }

    /// A tight list item has no paragraph events, so the construct itself
    /// must supply the boundary between the text either side of it.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn code_block_in_a_tight_list_item_is_a_boundary() {
        assert_eq!(
            extract("- Alpha one:\n  ```\n  code\n  ```\n  Beta two."),
            vec![
                text("Alpha one:", 2),
                ProseEvent::Boundary,
                text("Beta two.", 34),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn heading_in_a_tight_list_item_is_a_boundary() {
        assert_eq!(
            extract("- Alpha one:\n  # Heading here\n  Beta two."),
            vec![
                text("Alpha one:", 2),
                ProseEvent::Boundary,
                text("Beta two.", 32),
                ProseEvent::Boundary,
            ],
        );
    }

    /// An indented code block cannot interrupt a paragraph, so no document
    /// exists where it alone separates two text runs; a heading opens
    /// block context for it here. What this pins is the collapse: two
    /// adjacent construct boundaries yield one.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn adjacent_construct_boundaries_collapse_into_one() {
        assert_eq!(
            extract("- Alpha one:\n  # Heading\n      let x = 1;\n  Beta two."),
            vec![
                text("Alpha one:", 2),
                ProseEvent::Boundary,
                text("Beta two.", 44),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Spans are byte extents, so multi-byte characters must widen them.
    ///
    /// Author: Claude Fable 5
    #[test]
    fn non_ascii_text_keeps_byte_true_spans() {
        assert_eq!(
            extract("Début du texte.\n\nDeuxième phrase, voilà."),
            vec![
                text("Début du texte.", 0),
                ProseEvent::Boundary,
                text("Deuxième phrase, voilà.", 18),
                ProseEvent::Boundary,
            ],
        );
    }

    /// Author: Claude Fable 5
    #[test]
    fn empty_input_yields_no_events() {
        assert_eq!(extract(""), vec![]);
    }
}
