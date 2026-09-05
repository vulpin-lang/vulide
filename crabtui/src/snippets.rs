//! Static Vulpin code snippets, inserted at the cursor via the command
//! palette. Vulpin-only — offered when the active buffer's language is
//! `Language::Vulpin`.
//!
//! Every one of these is run against the real interpreter before landing
//! here, not guessed from the grammar. Two of the language's own footguns
//! are easy to get wrong and show up below: the `K` type char is a bareword
//! (`Kvar"prompt: "S`, not `Kvar"prompt: ""S"`), and there's deliberately no
//! `for` snippet — `O` currently heap-overflows on the interpreter's `General`
//! tip (reported upstream), so shipping one would hand people a crash.

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
        name: "while",
        body: "@ cond\n    \n&",
    },
    Snippet {
        name: "function",
        body: "F name(params)\n    \n~",
    },
    Snippet {
        name: "try / catch",
        body: "T\n    \nCerr\n    G err\nY",
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
        name: "input",
        body: "Kvar\"prompt: \"S",
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
];
