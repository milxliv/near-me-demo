//! Live lookup on Wikiquote: a line that uses the name, cited to the work and
//! speaker it comes from.
//!
//! Only the first word of the name leaves this service, as a search query to
//! en.wikiquote.org. Nothing about the visitor goes with it: no IP address,
//! no headers, no location. A failure, a timeout, or no usable line returns
//! `None`, and the caller falls back to the curated list.

use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;
use reqwest::{Client, Url, redirect};
use serde::Deserialize;
use tokio::sync::Semaphore;

use crate::QuoteResponse;
use crate::limit::RateLimiter;

pub const API: &str = "https://en.wikiquote.org/w/api.php";
/// Wikimedia asks API clients to name themselves and give a contact.
const USER_AGENT: &str = concat!(
    "near-me-quote/",
    env!("CARGO_PKG_VERSION"),
    " (https://github.com/milxliv/near-me-demo)"
);
/// The page gives up after 8 s, so the lookup gives up well before that.
pub const BUDGET: Duration = Duration::from_secs(4);
/// Upstream lookups across all visitors. Past this, the curated list answers.
pub const LOOKUPS_PER_MINUTE: u32 = 30;
const CONCURRENT_LOOKUPS: usize = 4;
/// Per response, after decompression.
const MAX_RESPONSE_BYTES: usize = 3 * 1024 * 1024;
const PAGES_PER_SEARCH: &str = "6";
const MIN_QUOTE_CHARS: usize = 20;
pub const MAX_QUOTE_CHARS: usize = 200;
pub const MAX_CITATION_CHARS: usize = 160;
/// Picks among this many of the best lines, so a name does not always get
/// the same one.
const PICK_FROM: usize = 3;
pub const CITATION_SUFFIX: &str = " · via Wikiquote";

/// Sections whose lines are not quotes by the page subject, or are not
/// reliably sourced.
const SKIPPED_SECTIONS: &[&str] = &[
    "about",
    "disputed",
    "misattributed",
    "unsourced",
    "attributed",
    "see also",
    "external links",
    "references",
    "cast",
    "sources",
    "notes",
    "further reading",
    "bibliography",
];
/// Section names that do not name a speaker.
const GENERIC_SECTIONS: &[&str] = &[
    "quotes",
    "dialogue",
    "other",
    "others",
    "main",
    "general",
    "misc",
    "miscellaneous",
    "tagline",
    "taglines",
];
/// A line containing a word that starts with one of these is dropped. This
/// keeps profanity, slurs, and graphic lines out of a page made for a friend.
/// It is a filter, not a guarantee.
const BLOCKED: &[&str] = &[
    "fuck", "shit", "cunt", "bitch", "bastard", "asshole", "dick", "cock", "pussy", "whore",
    "slut", "piss", "damn", "goddam", "nigg", "fag", "retard", "spic", "kike", "chink", "tranny",
    "rape", "kill", "murder", "strangl", "suicide", "shoot", "blood", "tortur", "corpse", "sex",
    "naked", "nude", "porn", "hitler", "nazi",
];

static SCREEN_TITLE: LazyLock<Regex> = LazyLock::new(|| {
    regex(concat!(
        r"\((?:\d{4} )?(?:film|TV series|miniseries|animated series|video game|musical|play",
        r"|season \d+|series \d+)\)$",
    ))
});
static PERSON_LEAD: LazyLock<Regex> = LazyLock::new(|| regex(r"\((?:born |c\. )?[^)]*\d{3,4}"));
static DIALOGUE: LazyLock<Regex> =
    LazyLock::new(|| regex(r"^(?:\*|:+)\s*'''([^']{1,40}?)'''\s*:?\s*(.*)$"));
static PLAIN_SPEAKER: LazyLock<Regex> = LazyLock::new(|| regex(r"^([A-Z][\w .'-]{0,30}):\s+(.+)$"));
static REF: LazyLock<Regex> = LazyLock::new(|| regex(r"<ref[^>]*/>|<ref[^>]*>.*?</ref>"));
static W_TEMPLATE: LazyLock<Regex> = LazyLock::new(|| regex(r"\{\{w\|(?:[^{}|]*\|)?([^{}|]*)\}\}"));
static WIKILINK: LazyLock<Regex> = LazyLock::new(|| regex(r"\[\[(?:[^\]|]*\|)?([^\]|]*)\]\]"));
static EXTLINK_TEXT: LazyLock<Regex> = LazyLock::new(|| regex(r"\[https?://\S+ ([^\]]*)\]"));
static EXTLINK_BARE: LazyLock<Regex> = LazyLock::new(|| regex(r"\[https?://\S+\]"));
static EMPHASIS: LazyLock<Regex> = LazyLock::new(|| regex(r"'{2,}"));
static BREAK: LazyLock<Regex> = LazyLock::new(|| regex(r"<br\s*/?>"));
static TAG: LazyLock<Regex> = LazyLock::new(|| regex(r"<[^>]*>"));
static STAGE: LazyLock<Regex> = LazyLock::new(|| regex(r"\[[^\]]*\]"));

/// Compiles a pattern that is a literal in this file; the unit tests compile
/// every one of them.
fn regex(pattern: &str) -> Regex {
    match Regex::new(pattern) {
        Ok(re) => re,
        Err(err) => panic!("invalid built-in pattern {pattern:?}: {err}"),
    }
}

pub struct Wikiquote {
    client: Client,
    api: Url,
    budget: Duration,
    limiter: RateLimiter,
    permits: Semaphore,
}

impl Wikiquote {
    pub fn new() -> anyhow::Result<Self> {
        Self::with_endpoint(API, BUDGET)
    }

    /// Uses another endpoint and time budget, for tests. Plain http is
    /// accepted only on loopback.
    pub fn with_endpoint(api: &str, budget: Duration) -> anyhow::Result<Self> {
        let api = Url::parse(api)?;
        let loopback = matches!(api.host_str(), Some("127.0.0.1" | "localhost"));
        anyhow::ensure!(
            api.scheme() == "https" || loopback,
            "Wikiquote endpoint must use https"
        );
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .redirect(redirect::Policy::none())
            .no_proxy()
            .connect_timeout(Duration::from_secs(2))
            .timeout(budget)
            .build()?;
        Ok(Self {
            client,
            api,
            budget,
            limiter: RateLimiter::new(
                LOOKUPS_PER_MINUTE,
                LOOKUPS_PER_MINUTE,
                Duration::from_secs(60),
            ),
            permits: Semaphore::new(CONCURRENT_LOOKUPS),
        })
    }

    /// Finds a line that uses `name` (as `clean_name` returns it). `seed`
    /// chooses among the best few.
    pub async fn find(&self, name: &str, seed: usize) -> Option<QuoteResponse> {
        let word = search_word(name)?;
        let _permit = self.permits.try_acquire().ok()?;
        if !self.limiter.allow("wikiquote") {
            return None;
        }
        let (film_query, any_query) = (format!("\"{word}\" intitle:film"), format!("\"{word}\""));
        let searches = async { tokio::join!(self.search(&film_query), self.search(&any_query)) };
        let (film, any) = tokio::time::timeout(self.budget, searches).await.ok()?;
        let pages: Vec<Page> = film
            .unwrap_or_default()
            .into_iter()
            .chain(any.unwrap_or_default())
            .collect();
        let mut found: Vec<Candidate> = Vec::new();
        for page in &pages {
            for candidate in candidates(&page.title, &page.wikitext, &word) {
                if !found.iter().any(|f| f.quote == candidate.quote) {
                    found.push(candidate);
                }
            }
        }
        choose(&found, seed)
    }

    /// Runs one search and returns the matching pages with their wikitext,
    /// best match first.
    async fn search(&self, query: &str) -> Option<Vec<Page>> {
        let mut url = self.api.clone();
        url.query_pairs_mut()
            .append_pair("action", "query")
            .append_pair("format", "json")
            .append_pair("formatversion", "2")
            .append_pair("generator", "search")
            .append_pair("gsrsearch", query)
            .append_pair("gsrnamespace", "0")
            .append_pair("gsrlimit", PAGES_PER_SEARCH)
            .append_pair("prop", "revisions")
            .append_pair("rvprop", "content")
            .append_pair("rvslots", "main");
        let mut response = self.client.get(url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.ok()? {
            if body.len() + chunk.len() > MAX_RESPONSE_BYTES {
                return None;
            }
            body.extend_from_slice(&chunk);
        }
        let reply: Reply = serde_json::from_slice(&body).ok()?;
        let mut pages: Vec<(u32, Page)> = reply
            .query?
            .pages
            .into_iter()
            .filter_map(|p| {
                let wikitext = p.revisions.into_iter().next()?.slots.main.content;
                Some((
                    p.index,
                    Page {
                        title: p.title,
                        wikitext,
                    },
                ))
            })
            .collect();
        pages.sort_by_key(|(index, _)| *index);
        Some(pages.into_iter().map(|(_, page)| page).collect())
    }
}

struct Page {
    title: String,
    wikitext: String,
}

#[derive(Deserialize)]
struct Reply {
    query: Option<ReplyQuery>,
}

#[derive(Deserialize)]
struct ReplyQuery {
    #[serde(default)]
    pages: Vec<ReplyPage>,
}

#[derive(Deserialize)]
struct ReplyPage {
    title: String,
    #[serde(default)]
    index: u32,
    #[serde(default)]
    revisions: Vec<Revision>,
}

#[derive(Deserialize)]
struct Revision {
    slots: Slots,
}

#[derive(Deserialize)]
struct Slots {
    main: Slot,
}

#[derive(Deserialize)]
struct Slot {
    #[serde(default)]
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub quote: String,
    pub citation: String,
    /// From a film or TV page. These are ranked first.
    pub screen: bool,
}

/// The first word of the name as it would appear in a line:
/// "jack smith" -> "Jack", "McKenzie" stays "McKenzie".
pub fn search_word(name: &str) -> Option<String> {
    let word = name
        .split(' ')
        .next()?
        .trim_end_matches(['.', '\'', '\u{2019}', '-']);
    if word.chars().count() < 2 {
        return None;
    }
    let one_case =
        word.chars().all(|c| !c.is_uppercase()) || word.chars().all(|c| !c.is_lowercase());
    let mut chars = word.chars();
    let first = chars.next()?;
    let rest: String = if one_case {
        chars.flat_map(char::to_lowercase).collect()
    } else {
        chars.collect()
    };
    Some(first.to_uppercase().chain(rest.chars()).collect())
}

/// Lines on one page that use `word`, with their citations.
pub fn candidates(title: &str, wikitext: &str, word: &str) -> Vec<Candidate> {
    let screen = SCREEN_TITLE.is_match(title) || wikitext.contains("\n== Cast ==");
    let person = !screen
        && wikitext
            .lines()
            .find(|l| l.starts_with("'''"))
            .is_some_and(|lead| PERSON_LEAD.is_match(&lead.chars().take(300).collect::<String>()));
    let lines: Vec<&str> = wikitext.lines().collect();
    let mut sections: Vec<String> = Vec::new();
    let mut found = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if let Some((level, text)) = heading(line) {
            sections.truncate(level.saturating_sub(2));
            sections.push(clean(text));
            continue;
        }
        if sections.iter().any(|s| skipped(s)) {
            continue;
        }
        let dialogue = if screen {
            DIALOGUE.captures(line)
        } else {
            None
        };
        let (mut speaker, mut text) = if let Some(c) = dialogue {
            let who = clean(&c[1]).trim_end_matches(':').trim().to_owned();
            (Some(who), spoken(&c[2]))
        } else if line.starts_with('*') && !line.starts_with("**") && !line.starts_with("*:") {
            (None, spoken(&line[1..]))
        } else {
            continue;
        };
        // Screen pages also write dialogue as "* Name: line" without bold.
        if screen
            && speaker.is_none()
            && let Some(c) = PLAIN_SPEAKER.captures(&text)
        {
            let (who, said) = (c[1].to_owned(), c[2].to_owned());
            speaker = Some(who);
            text = said;
        }
        let length = text.chars().count();
        if !(MIN_QUOTE_CHARS..=MAX_QUOTE_CHARS).contains(&length)
            || text.contains(['{', '}', '|', '<', '>', '=', '[', ']'])
            || !uses_name(&text, word)
            || blocked(&text)
        {
            continue;
        }

        let citation = if screen {
            // A top-level section on a film page is usually a character name.
            let section = match sections.as_slice() {
                [only] if !generic(only) => Some(only.as_str()),
                _ => None,
            };
            match speaker.as_deref().filter(|s| !s.is_empty()).or(section) {
                Some(who) => format!("{who}, {title}"),
                None => title.to_owned(),
            }
        } else {
            // Elsewhere a quote counts only with its source line under it.
            let Some(source) = lines
                .get(i + 1)
                .filter(|l| l.starts_with("**"))
                .map(|l| clean(&l[2..]))
            else {
                continue;
            };
            let names_a_work = source.chars().filter(|c| c.is_alphabetic()).count() >= 3;
            if person || !names_a_work {
                format!("{title}, {source}")
            } else {
                source
            }
        };
        if citation.chars().count() > MAX_CITATION_CHARS
            || citation.contains(['{', '}', '|', '<', '>', '[', ']'])
        {
            continue;
        }
        found.push(Candidate {
            quote: text,
            citation,
            screen,
        });
    }
    found
}

/// Film and TV lines first, short lines before long ones; then one of the
/// first few in page order.
fn choose(found: &[Candidate], seed: usize) -> Option<QuoteResponse> {
    let tier = |c: &Candidate| match (c.screen, c.quote.chars().count() <= 100) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3,
    };
    let best = found.iter().map(tier).min()?;
    let top: Vec<&Candidate> = found
        .iter()
        .filter(|c| tier(c) == best)
        .take(PICK_FROM)
        .collect();
    let chosen = top.get(seed % top.len())?;
    Some(QuoteResponse {
        quote: chosen.quote.clone(),
        citation: format!("{}{CITATION_SUFFIX}", chosen.citation),
    })
}

/// `== Heading ==` -> (2, "Heading").
fn heading(line: &str) -> Option<(usize, &str)> {
    let line = line.trim_end();
    let open = line.chars().take_while(|&c| c == '=').count();
    let close = line.chars().rev().take_while(|&c| c == '=').count();
    if open < 2 || close < 2 || line.len() <= open + close {
        return None;
    }
    Some((open.min(close), line[open..line.len() - close].trim()))
}

/// True when `word` appears as a whole word and not just as the capitalized
/// first word of a sentence or verse line ("Will you…" is not about Will; "Will, come
/// here" is).
fn uses_name(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(at, _)| {
        let before = &text[..at];
        let after = &text[at + word.len()..];
        let joined_before = before
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '\'' | '\u{2019}' | '-'));
        let joined_after = after
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || c == '-');
        if joined_before || joined_after {
            return false;
        }
        let lead = before
            .trim_end_matches(|c: char| c.is_whitespace() || "\"'\u{201c}\u{2018}".contains(c));
        let starts_sentence = lead.is_empty() || lead.ends_with(['.', '!', '?', '/']);
        !starts_sentence || after.starts_with(',')
    })
}

fn blocked(text: &str) -> bool {
    let lower = text.to_lowercase();
    BLOCKED.iter().any(|term| {
        lower.match_indices(term).any(|(at, _)| {
            !lower[..at]
                .chars()
                .next_back()
                .is_some_and(char::is_alphabetic)
        })
    })
}

fn skipped(section: &str) -> bool {
    let lower = section.to_lowercase();
    SKIPPED_SECTIONS.iter().any(|name| lower.contains(name))
}

fn generic(section: &str) -> bool {
    let lower = section.to_lowercase();
    GENERIC_SECTIONS.contains(&lower.as_str()) || lower.starts_with("season ")
}

/// Spoken text: markup removed, then stage directions such as `[exits]`.
fn spoken(raw: &str) -> String {
    let text = clean(raw);
    let text = STAGE.replace_all(&text, " ");
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Wikitext to plain text.
fn clean(raw: &str) -> String {
    let s = REF.replace_all(raw, "");
    let s = W_TEMPLATE.replace_all(&s, "$1");
    let s = strip_templates(&s);
    let s = WIKILINK.replace_all(&s, "$1");
    let s = EXTLINK_TEXT.replace_all(&s, "$1");
    let s = EXTLINK_BARE.replace_all(&s, "");
    let s = EMPHASIS.replace_all(&s, "");
    // Verse lines are joined with <br>; quote them with " / ".
    let s = BREAK.replace_all(&s, " / ");
    let s = TAG.replace_all(&s, "");
    let s = decode_entities(&s);
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_templates(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut depth = 0usize;
    let mut rest = s;
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix("{{") {
            depth += 1;
            rest = after;
        } else if let Some(after) = rest.strip_prefix("}}").filter(|_| depth > 0) {
            depth -= 1;
            rest = after;
        } else {
            let mut chars = rest.chars();
            if let Some(c) = chars.next().filter(|_| depth == 0) {
                out.push(c);
            }
            rest = chars.as_str();
        }
    }
    out
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let decoded = rest
            .find(';')
            .filter(|&end| end <= 10)
            .and_then(|end| entity(&rest[1..end]).map(|c| (c, end)));
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    let named = match name {
        "amp" => '&',
        "quot" => '"',
        "apos" => '\'',
        "lt" => '<',
        "gt" => '>',
        "nbsp" => ' ',
        "ndash" => '\u{2013}',
        "mdash" => '\u{2014}',
        "hellip" => '\u{2026}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "ldquo" => '\u{201c}',
        "rdquo" => '\u{201d}',
        _ => {
            let code = match name.strip_prefix("#x").or_else(|| name.strip_prefix("#X")) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => name.strip_prefix('#')?.parse().ok()?,
            };
            return char::from_u32(code);
        }
    };
    Some(named)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic pages in Wikiquote's format. No real page text.
    const FILM: &str = "\
'''Harbor Lights''' is a 1999 film about a lighthouse keeper.
== Maggie ==
* Keep the lamp lit, [[Tom]]. Somebody out there needs it.
* The sea doesn't care, Tom.
== Dialogue ==
:'''Captain Reyes''': [hands Tom the logbook] You're late again, Tom.
:'''Tom''': I know, I know. The fog was thick tonight.
* '''Old Pete''': Tom, the gulls know more than you do.
* Pete: Tom, you never listen to anybody around here.
:'''Maggie''': Tom, they're going to kill us all if the lamp goes out!
== Taglines ==
* One light. One keeper. One night, Tom.
== Cast ==
* Tom Hale as Tom
";

    const THEME: &str = "\
'''Patience''' is the capacity to wait.
== Quotes ==
* Tom waited by the door for an hour, and the hour waited with him.
** [[Ada Moss]], ''The Long Porch'' (1931), ch. 2
* Tom is a fine name for a cat with no patience at all.
* Wait for it, Tom. Wait for it.
** {{w|Lena Ortiz|Lena Ortiz}}, ''Field Notes''<ref>p. 4</ref>
== Quotes about patience ==
* Tom knew patience better than anyone I met.
** Somebody, somewhere
";

    const PERSON: &str = "\
'''Ada Moss''' (3 May 1890 – 1 June 1960) was a writer.
== Quotes ==
* Bring the boat around, Tom, and mind the reeds.
** ''The Long Porch'' (1931)
== Disputed ==
* Tom, that's not what I said at all.
** Attributed in a 1990 newsletter
";

    fn quotes(found: &[Candidate]) -> Vec<&str> {
        found.iter().map(|c| c.quote.as_str()).collect()
    }

    #[test]
    fn built_in_patterns_compile() {
        for re in [
            &SCREEN_TITLE,
            &PERSON_LEAD,
            &DIALOGUE,
            &PLAIN_SPEAKER,
            &REF,
            &W_TEMPLATE,
            &WIKILINK,
            &EXTLINK_TEXT,
            &EXTLINK_BARE,
            &EMPHASIS,
            &BREAK,
            &TAG,
            &STAGE,
        ] {
            LazyLock::force(re);
        }
    }

    #[test]
    fn film_lines_cite_the_speaker() {
        let found = candidates("Harbor Lights (1999 film)", FILM, "Tom");
        let pairs: Vec<(&str, &str)> = found
            .iter()
            .map(|c| (c.quote.as_str(), c.citation.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![
                (
                    "Keep the lamp lit, Tom. Somebody out there needs it.",
                    "Maggie, Harbor Lights (1999 film)"
                ),
                (
                    "The sea doesn't care, Tom.",
                    "Maggie, Harbor Lights (1999 film)"
                ),
                (
                    "You're late again, Tom.",
                    "Captain Reyes, Harbor Lights (1999 film)"
                ),
                (
                    "Tom, the gulls know more than you do.",
                    "Old Pete, Harbor Lights (1999 film)"
                ),
                (
                    "Tom, you never listen to anybody around here.",
                    "Pete, Harbor Lights (1999 film)"
                ),
                (
                    "One light. One keeper. One night, Tom.",
                    "Harbor Lights (1999 film)"
                ),
            ]
        );
        assert!(found.iter().all(|c| c.screen));
    }

    #[test]
    fn the_name_must_be_in_the_spoken_words() {
        // "Tom" appears only as the speaker or in a stage direction.
        let found = candidates("Harbor Lights (1999 film)", FILM, "Tom");
        assert!(!quotes(&found).iter().any(|q| q.contains("fog")));
        assert!(!quotes(&found).iter().any(|q| q.contains("logbook")));
    }

    #[test]
    fn blocked_words_drop_a_line() {
        let found = candidates("Harbor Lights (1999 film)", FILM, "Tom");
        assert!(!quotes(&found).iter().any(|q| q.contains("kill")));
        assert!(blocked("Don't shoot!"));
        assert!(!blocked("That takes real skill, Tom."));
    }

    #[test]
    fn theme_pages_need_a_source_and_skip_quotes_about() {
        let found = candidates("Patience", THEME, "Tom");
        let pairs: Vec<(&str, &str)> = found
            .iter()
            .map(|c| (c.quote.as_str(), c.citation.as_str()))
            .collect();
        assert_eq!(
            pairs,
            vec![("Wait for it, Tom. Wait for it.", "Lena Ortiz, Field Notes")]
        );
        // The first quote starts a sentence with the name; the second has no source.
        assert!(!found.iter().any(|c| c.screen));
    }

    #[test]
    fn person_pages_cite_the_person_and_skip_disputed() {
        let found = candidates("Ada Moss", PERSON, "Tom");
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(
            found[0].quote,
            "Bring the boat around, Tom, and mind the reeds."
        );
        assert_eq!(found[0].citation, "Ada Moss, The Long Porch (1931)");
    }

    #[test]
    fn choose_prefers_short_screen_lines() {
        let line = |quote: &str, screen| Candidate {
            quote: quote.to_owned(),
            citation: "c".to_owned(),
            screen,
        };
        let long = "x".repeat(120);
        let found = vec![
            line("theme line, Tom", false),
            line(&long, true),
            line("short screen line, Tom", true),
        ];
        let chosen = choose(&found, 7).expect("a pick");
        assert_eq!(chosen.quote, "short screen line, Tom");
        assert_eq!(chosen.citation, "c · via Wikiquote");
        assert!(choose(&[], 0).is_none());
    }

    #[test]
    fn search_word_normalizes_case_and_takes_the_first_word() {
        assert_eq!(search_word("jack smith").as_deref(), Some("Jack"));
        assert_eq!(search_word("JACK").as_deref(), Some("Jack"));
        assert_eq!(search_word("McKenzie").as_deref(), Some("McKenzie"));
        assert_eq!(search_word("José").as_deref(), Some("José"));
        assert_eq!(search_word("J. R.").as_deref(), None);
    }

    #[test]
    fn uses_name_needs_a_whole_word_not_a_sentence_start() {
        assert!(uses_name("I'm flying, Jack!", "Jack"));
        assert!(uses_name("Jack, come here.", "Jack"));
        assert!(uses_name("Is that Jack's hat?", "Jack"));
        assert!(!uses_name("Jack went home.", "Jack"));
        assert!(!uses_name("It was late. Will you stay?", "Will"));
        assert!(!uses_name(
            "The chemist of love / Will this perishing mould",
            "Will"
        ));
        assert!(!uses_name("Jackson called.", "Jack"));
        assert!(!uses_name("a jack-in-the-box", "Jack"));
    }

    #[test]
    fn clean_removes_markup() {
        assert_eq!(
            clean("''[[Moby-Dick|The whale]]'' said '''hi'''<ref name=\"a\"/> {{cite|x}}"),
            "The whale said hi"
        );
        assert_eq!(
            clean("{{w|Ada Moss}} &amp; [https://example.com Tom]"),
            "Ada Moss & Tom"
        );
        assert_eq!(
            clean("one<br />two&nbsp;three &#8212; &bogus;"),
            "one / two three \u{2014} &bogus;"
        );
    }

    #[test]
    fn heading_levels() {
        assert_eq!(heading("== Maggie =="), Some((2, "Maggie")));
        assert_eq!(heading("=== Season 1 ==="), Some((3, "Season 1")));
        assert_eq!(heading("* not = a heading"), None);
        assert_eq!(heading("===="), None);
    }
}
