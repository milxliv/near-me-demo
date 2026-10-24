//! Curated quotes that use a first name, plus adapted fallbacks.
//!
//! This is the fallback when the Wikiquote lookup finds nothing, fails, or is
//! switched off. The list is fixed and reviewed by hand.

use crate::QuoteResponse;

pub const MAX_NAME_CHARS: usize = 40;

pub struct Quote {
    pub text: &'static str,
    pub citation: &'static str,
}

const fn q(text: &'static str, citation: &'static str) -> Quote {
    Quote { text, citation }
}

/// Real lines that address or use the name, keyed by lowercase first name.
const NAMED: &[(&str, &[Quote])] = &[
    ("adrian", &[q("Yo, Adrian!", "Rocky Balboa, Rocky (1976)")]),
    (
        "bob",
        &[q(
            "Bob's your uncle.",
            "British saying for \"and there you have it\"",
        )],
    ),
    (
        "brutus",
        &[q(
            "Et tu, Brute?",
            "Julius Caesar, Shakespeare, Julius Caesar, Act 3, Scene 1",
        )],
    ),
    (
        "carl",
        &[q(
            "Carl, that kills people!",
            "Paul, Llamas with Hats (FilmCow, 2009)",
        )],
    ),
    (
        "charlie",
        &[q(
            "Charlie bit me!",
            "Harry, \"Charlie Bit My Finger – Again!\" (YouTube, 2007)",
        )],
    ),
    (
        "dave",
        &[q(
            "I'm sorry, Dave. I'm afraid I can't do that.",
            "HAL 9000, 2001: A Space Odyssey (1968)",
        )],
    ),
    (
        "david",
        &[q(
            "I'm sorry, Dave. I'm afraid I can't do that.",
            "HAL 9000, 2001: A Space Odyssey (1968)",
        )],
    ),
    (
        "elvis",
        &[q(
            "Elvis has left the building.",
            "Horace Lee Logan, Louisiana Hayride (1956)",
        )],
    ),
    (
        "forrest",
        &[q("Run, Forrest, run!", "Jenny Curran, Forrest Gump (1994)")],
    ),
    (
        "fredo",
        &[q(
            "I know it was you, Fredo.",
            "Michael Corleone, The Godfather Part II (1974)",
        )],
    ),
    (
        "hal",
        &[q(
            "Open the pod bay doors, HAL.",
            "Dave Bowman, 2001: A Space Odyssey (1968)",
        )],
    ),
    (
        "harry",
        &[q(
            "You're a wizard, Harry.",
            "Rubeus Hagrid, Harry Potter and the Sorcerer's Stone (2001)",
        )],
    ),
    (
        "jack",
        &[
            q("I'm flying, Jack!", "Rose DeWitt Bukater, Titanic (1997)"),
            q(
                "All work and no play makes Jack a dull boy.",
                "Proverb (James Howell, 1659), typed over and over in The Shining (1980)",
            ),
            q(
                "Hit the road, Jack!",
                "\"Hit the Road Jack,\" written by Percy Mayfield, sung by Ray Charles (1961)",
            ),
        ],
    ),
    (
        "james",
        &[q("Bond. James Bond.", "James Bond, Dr. No (1962)")],
    ),
    (
        "jenny",
        &[q(
            "Jenny and me was like peas and carrots.",
            "Forrest Gump, Forrest Gump (1994)",
        )],
    ),
    (
        "jill",
        &[q(
            "Jack and Jill went up the hill to fetch a pail of water.",
            "Traditional nursery rhyme",
        )],
    ),
    (
        "johnny",
        &[q(
            "Here's Johnny!",
            "Jack Torrance, The Shining (1980), borrowing the Tonight Show introduction",
        )],
    ),
    (
        "juliet",
        &[q(
            "It is the east, and Juliet is the sun.",
            "Romeo, Shakespeare, Romeo and Juliet, Act 2, Scene 2",
        )],
    ),
    (
        "kevin",
        &[q("KEVIN!", "Kate McCallister, Home Alone (1990)")],
    ),
    (
        "leeroy",
        &[q(
            "Leeroy Jenkins!",
            "Leeroy Jenkins, World of Warcraft raid video (2005)",
        )],
    ),
    (
        "louis",
        &[q(
            "Louis, I think this is the beginning of a beautiful friendship.",
            "Rick Blaine, Casablanca (1942)",
        )],
    ),
    (
        "luke",
        &[q(
            "Use the Force, Luke.",
            "Obi-Wan Kenobi, Star Wars (1977)",
        )],
    ),
    (
        "mary",
        &[q(
            "Mary had a little lamb, its fleece was white as snow.",
            "Sarah Josepha Hale, \"Mary's Lamb\" (1830)",
        )],
    ),
    (
        "newman",
        &[q(
            "Hello, Newman.",
            "Jerry Seinfeld, Seinfeld (NBC, 1989-1998)",
        )],
    ),
    (
        "peter",
        &[q(
            "Peter Piper picked a peck of pickled peppers.",
            "Tongue twister, Peter Piper's Practical Principles of Plain and Perfect \
             Pronunciation (1813)",
        )],
    ),
    (
        "romeo",
        &[q(
            "O Romeo, Romeo, wherefore art thou Romeo?",
            "Juliet, Shakespeare, Romeo and Juliet, Act 2, Scene 2",
        )],
    ),
    (
        "rose",
        &[q(
            "What's in a name? That which we call a rose by any other name would smell as sweet.",
            "Juliet, Shakespeare, Romeo and Juliet, Act 2, Scene 2",
        )],
    ),
    (
        "sam",
        &[q(
            "Play it, Sam. Play 'As Time Goes By.'",
            "Ilsa Lund, Casablanca (1942)",
        )],
    ),
    (
        "stella",
        &[q(
            "Stella! Hey, Stella!",
            "Stanley Kowalski, A Streetcar Named Desire (1951)",
        )],
    ),
    (
        "toto",
        &[q(
            "Toto, I've a feeling we're not in Kansas anymore.",
            "Dorothy Gale, The Wizard of Oz (1939)",
        )],
    ),
    (
        "watson",
        &[q(
            "Mr. Watson, come here, I want to see you.",
            "Alexander Graham Bell, the first telephone call (1876)",
        )],
    ),
    (
        "will",
        &[q(
            "Danger, Will Robinson!",
            "The Robot, Lost in Space (1965-1968)",
        )],
    ),
];

/// Famous lines with the name put in. The citation says "Adapted from" so
/// nobody mistakes them for the original.
const ADAPTED: &[(&str, &str)] = &[
    (
        "{name}, I've a feeling we're not in Kansas anymore.",
        "Adapted from Dorothy Gale, The Wizard of Oz (1939)",
    ),
    (
        "Here's looking at you, {name}.",
        "Adapted from Rick Blaine, Casablanca (1942)",
    ),
    (
        "May the Force be with you, {name}.",
        "Adapted from Star Wars (1977)",
    ),
    (
        "{name}, you're gonna need a bigger boat.",
        "Adapted from Martin Brody, Jaws (1975)",
    ),
    (
        "To infinity and beyond, {name}!",
        "Adapted from Buzz Lightyear, Toy Story (1995)",
    ),
];

/// Normalizes a submitted name, or rejects it.
///
/// Accepts 1-40 characters: letters, spaces, hyphens, apostrophes, and
/// periods, starting with a letter. Runs of whitespace collapse to one space.
pub fn clean_name(raw: &str) -> Option<String> {
    let name = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let first = name.chars().next()?;
    let allowed = |c: char| c.is_alphabetic() || matches!(c, ' ' | '-' | '\'' | '\u{2019}' | '.');
    if !first.is_alphabetic() || name.chars().count() > MAX_NAME_CHARS || !name.chars().all(allowed)
    {
        return None;
    }
    Some(name)
}

/// Picks a quote for a name that `clean_name` accepted. `seed` chooses among
/// several candidates.
pub fn pick(name: &str, seed: usize) -> QuoteResponse {
    let key = lookup_key(name);
    if let Some((_, quotes)) = NAMED.iter().find(|(k, _)| *k == key) {
        let chosen = &quotes[seed % quotes.len()];
        return QuoteResponse {
            quote: chosen.text.to_owned(),
            citation: chosen.citation.to_owned(),
        };
    }
    let (template, citation) = ADAPTED[seed % ADAPTED.len()];
    QuoteResponse {
        quote: template.replace("{name}", &display_name(name)),
        citation: citation.to_owned(),
    }
}

/// "Mary-Jane Smith" -> "mary".
fn lookup_key(name: &str) -> String {
    name.split(|c: char| !c.is_alphabetic())
        .next()
        .unwrap_or_default()
        .chars()
        .flat_map(char::to_lowercase)
        .collect()
}

/// Capitalizes the first letter of each word and keeps the rest as typed.
fn display_name(name: &str) -> String {
    name.split(' ')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().chain(chars).collect(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_name_trims_and_collapses_spaces() {
        assert_eq!(clean_name("  Jack  ").as_deref(), Some("Jack"));
        assert_eq!(clean_name("Mary \t Jane").as_deref(), Some("Mary Jane"));
        assert_eq!(clean_name("José").as_deref(), Some("José"));
        assert_eq!(clean_name("O'Brien").as_deref(), Some("O'Brien"));
        assert_eq!(clean_name(&"a".repeat(40)).map(|n| n.len()), Some(40));
    }

    #[test]
    fn clean_name_rejects_everything_else() {
        for bad in [
            "",
            "   ",
            "<script>",
            "1jack",
            "-jack",
            "jack1",
            "jack\u{0}",
            "jack\u{200d}",
            "a@b.c",
            "{name}",
        ] {
            assert_eq!(clean_name(bad), None, "{bad:?} should be rejected");
        }
        assert_eq!(clean_name(&"a".repeat(41)), None);
    }

    #[test]
    fn known_names_get_a_real_quote() {
        for seed in 0..3 {
            let r = pick("jack", seed);
            assert!(r.quote.contains("Jack"), "{}", r.quote);
            assert!(!r.citation.starts_with("Adapted"), "{}", r.citation);
        }
        assert_eq!(pick("Johnny Smith", 0).quote, "Here's Johnny!");
        assert_eq!(pick("Mary-Jane", 0).quote, NAMED_MARY);
    }

    const NAMED_MARY: &str = "Mary had a little lamb, its fleece was white as snow.";

    #[test]
    fn unknown_names_get_an_adapted_quote_with_the_name() {
        for seed in 0..ADAPTED.len() {
            let r = pick("zebulon ann", seed);
            assert!(r.quote.contains("Zebulon Ann"), "{}", r.quote);
            assert!(r.citation.starts_with("Adapted from "), "{}", r.citation);
        }
    }

    #[test]
    fn tables_are_well_formed() {
        for (key, quotes) in NAMED {
            assert!(!quotes.is_empty(), "{key}");
            assert!(key.chars().all(|c| c.is_ascii_lowercase()), "{key}");
            for quote in *quotes {
                assert!(
                    !quote.text.is_empty() && !quote.citation.is_empty(),
                    "{key}"
                );
            }
        }
        let mut keys: Vec<_> = NAMED.iter().map(|(k, _)| *k).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), NAMED.len(), "duplicate key");
        for (template, citation) in ADAPTED {
            assert!(template.contains("{name}"), "{template}");
            assert!(citation.starts_with("Adapted from "), "{citation}");
        }
    }
}
