//! Static Vulpin code snippets, inserted at the cursor via the command
//! palette. Vulpin-only — offered when the active buffer's language is
//! `Language::Vulpin`.
//!
//! Every one of these is run against the real interpreter before landing
//! here, not guessed from the grammar. One footgun shows up below: the `K`
//! type char must be a *string* (`Kn"number: ""I"`) — the VM ignores a bare
//! `I`, so `Kn"number: "I` silently reads a string.

pub struct Snippet {
    pub name: &'static str,
    pub body: &'static str,
}

pub const SNIPPETS: &[Snippet] = &[
    Snippet {
        name: "if",
        body: "? cond\n    \n;",
    },
    Snippet {
        name: "if / else",
        body: "? cond\n    \n:\n    \n;",
    },
    Snippet {
        name: "if / else if / else",
        body: "? cond\n    \n:? cond\n    \n:\n    \n;",
    },
    Snippet {
        name: "while",
        body: "@ cond\n    \n&",
    },
    Snippet {
        name: "for",
        body: "O i 0 10\n    G i\n&",
    },
    Snippet {
        name: "function",
        body: "F name(params)\n    \n~",
    },
    Snippet {
        name: "try / catch",
        body: "T\n    \nC\"err\"\n    G err\nY",
    },
    Snippet {
        name: "switch",
        body: "W expr\n    V value\n        \n    N\n        \nZ",
    },
    Snippet {
        name: "print",
        body: "G\"\"",
    },
    Snippet {
        name: "format string",
        body: "G $\"Hello {name}\"",
    },
    Snippet {
        name: "input",
        body: "Kname\"prompt: \"",
    },
    Snippet {
        name: "input (integer)",
        body: "Kn\"number: \"\"I\"",
    },
    Snippet {
        name: "list comprehension",
        body: "[x * 2 | x <- items]",
    },
    Snippet {
        name: "import",
        body: "U\"module\"",
    },
    Snippet {
        name: "label",
        body: "L name",
    },
    Snippet {
        name: "jump",
        body: "J name",
    },
    Snippet {
        name: "exit with code",
        body: "Q 1",
    },
];
