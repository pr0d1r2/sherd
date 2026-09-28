use super::*;

#[test]
fn a_query_naming_one_node_resolves_to_it() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Route::Hit(node, why) =
        route(root, "how does the ollama endpoint retry")
    else {
        unreachable!("`ollama` names exactly one node")
    };
    assert!(node.ends_with("ollama"), "{}", node.display());
    assert!(why.contains(&"ollama".to_string()), "the reason: {why:?}");
}

/// A miss is REPORTED, never rounded to the nearest node. The query's
/// author can read a miss and rephrase; a confident wrong node sends
/// them to read the wrong file.
#[test]
fn a_query_naming_nothing_is_a_miss() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_eq!(route(root, "wombat marmalade trebuchet"), Route::Miss);
}

/// A tie is genuinely ambiguous, and saying so beats picking the first.
#[test]
fn a_query_spanning_two_nodes_equally_is_ambiguous() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let Route::Ambiguous(nodes) = route(root, "ollama tokens") else {
        unreachable!("one word each from two nodes is a tie")
    };
    assert!(nodes.len() >= 2, "{nodes:?}");
}

/// The ROOT owns everything and therefore answers nothing.
#[test]
fn the_root_is_never_the_answer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    for q in ["federation", "spec", "token"] {
        match route(root, q) {
            Route::Hit(node, _) => assert_ne!(node, root, "{q}"),
            Route::Ambiguous(nodes) => {
                assert!(!nodes.contains(&root.to_path_buf()), "{q}");
            }
            Route::Miss => {}
        }
    }
}

/// Short words are dropped: they are caveman prose's articles, and one
/// of them would match every node that ever used it.
#[test]
fn words_under_four_characters_carry_no_signal() {
    assert!(goal_words("## \u{a7}G GOAL\n\na of the is\n").is_empty());
}

/// The §G half has to be REAL, not merely non-empty: the directory name
/// alone satisfies "has words", and it did while `goal_words` silently
/// returned nothing for every node.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "the count and the CONTENT are one property here: a \
                  vocabulary of the right size built from directory names \
                  alone is exactly the defect this asserts against"
)]
fn a_vocabulary_is_derived_for_every_node_the_walk_finds() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let vocab = vocabulary(root);
    assert_eq!(vocab.len(), fed::discover(root).len());
    assert!(
        vocab.iter().all(|(_, w)| !w.is_empty()),
        "a node with no words can never be routed to"
    );
    let fed_words = vocab
        .iter()
        .find(|(n, _)| n.ends_with("fed"))
        .map(|(_, w)| w.clone())
        .unwrap_or_default();
    assert!(
        fed_words.iter().any(|w| w == "federation"),
        "§G's own words reach the vocabulary: {fed_words:?}"
    );
}
