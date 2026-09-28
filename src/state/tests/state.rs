use super::*;

/// `cached_tokens` is a CACHE, and a cache that never hits is a slow
/// counter while a cache that never misses is a wrong one.
///
/// `State::at` rather than `State::load`, so this touches no ambient
/// `.sherd-state`. `.coverage` records the suite's coverage flapping
/// because tests share that one file, and adding another writer to it
/// would make a measurement problem worse to fix a coverage number.
#[test]
fn counting_a_file_twice_hits_the_cache_and_a_changed_file_misses() {
    assert_eq!(cache_hits_then_misses(), Ok(()));
}

/// A file and a state file, both unique per INSTANCE (`src/review:V6`).
fn scratch_pair(tag: &str) -> (std::path::PathBuf, std::path::PathBuf) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let d = std::env::temp_dir();
    let pid = std::process::id();
    (
        d.join(format!("sherd-{tag}-{pid}-{n}.md")),
        d.join(format!("sherd-{tag}-{pid}-{n}.state")),
    )
}

/// V2: `set` -> `save` -> `load` -> `get` returns what was set.
///
/// B1 is the absence of this. Every loop label contains a space --
/// `1 red-test`, `2 green`, `5 blind judge` -- so the per-label pace
/// model was WRITE-ONLY from the day it was built, and `predict_for`
/// silently fell back to the default on every step.
#[test]
fn a_key_with_a_space_survives_the_round_trip() {
    assert_eq!(round_trips(), Ok(()));
}

/// Every shape a key takes here, plus the escape character itself.
const KEYS: &[(&str, &str, &str)] = &[
    ("gen", "1 red-test", "2342"),
    ("gen", "5 blind judge", "700"),
    ("gen", "monolith", "999"),
    ("score", "src/fed.tried", "3"),
    ("gen", "100% sure", "1"),
    ("gen", "already %20 escaped", "2"),
];

fn round_trips() -> Result<(), String> {
    let (_f, p) = scratch_pair("roundtrip");
    let mut st = State::at(&p);
    for (kind, key, val) in KEYS {
        st.set(kind, key, *val);
    }
    st.save();
    assert_all_readable(&State::at(&p));
    let _ = std::fs::remove_file(&p);
    Ok(())
}

fn assert_all_readable(back: &State) {
    for (kind, key, val) in KEYS {
        assert_eq!(
            back.get(kind, key),
            Some(*val),
            "`{key}` must read back as it was written"
        );
    }
}

#[test]
fn a_value_may_still_contain_spaces() {
    // The reason the key is the middle field and not the last: an `obs`
    // row's VALUE is a whole telemetry line. Escaping the key must not
    // disturb that.
    let (_f, p) = scratch_pair("obsvalue");
    let mut st = State::at(&p);
    st.set("obs", "deadbeef", "1 red-test 8377 900 2342 400 0");
    st.save();
    assert_eq!(
        State::at(&p).get("obs", "deadbeef"),
        Some("1 red-test 8377 900 2342 400 0"),
        "the value keeps its spaces"
    );
    let _ = std::fs::remove_file(&p);
}

fn cache_hits_then_misses() -> Result<(), String> {
    let (f, s) = scratch_pair("cache");
    std::fs::write(&f, "hello world").map_err(|e| e.to_string())?;
    let mut st = State::at(&s);
    let first = cached_tokens(&mut st, &f).map_err(|e| e.to_string())?;
    let key = f.to_string_lossy().to_string();
    assert!(st.get("hash", &key).is_some(), "the hash is recorded");
    let again = cached_tokens(&mut st, &f).map_err(|e| e.to_string())?;
    assert_eq!(first, again, "identical content, identical count");
    // Changed content must MISS -- a cache keyed on the path alone would
    // return a stale count for an edited file, and every budget and
    // ceiling in this repo is computed from these numbers.
    std::fs::write(&f, "hello world, and a good deal more text besides")
        .map_err(|e| e.to_string())?;
    let after = cached_tokens(&mut st, &f).map_err(|e| e.to_string())?;
    assert!(after > first, "an edited file recounts: {first} -> {after}");
    let _ = std::fs::remove_file(&f);
    let _ = std::fs::remove_file(&s);
    Ok(())
}

#[test]
fn an_unreadable_file_is_an_error_not_a_zero() {
    // V48 through `cached_tokens`: a file that cannot be read must not
    // silently contribute zero tokens to a budget.
    let mut st = State::at(std::env::temp_dir().join("sherd-nonexistent"));
    let missing = std::path::Path::new("/definitely/not/here.md");
    assert!(cached_tokens(&mut st, missing).is_err());
}

#[test]
fn round_trips_and_is_idempotent() {
    let mut a = State::default();
    a.set("pace", "prefill", "1738");
    a.set("tok", "src/fed/SPEC.md", "1023");
    let once = a.serialise();
    let twice = a.clone().serialise();
    assert_eq!(
        once, twice,
        "serialising unchanged state must be byte-identical"
    );
    assert_eq!(a.get("pace", "prefill"), Some("1738"));
    assert_eq!(a.get_u64("tok", "src/fed/SPEC.md"), Some(1023));
}

#[test]
fn ordering_is_stable_regardless_of_insertion_order() {
    let mut a = State::default();
    a.set("z", "k", "1");
    a.set("a", "k", "2");
    let mut b = State::default();
    b.set("a", "k", "2");
    b.set("z", "k", "1");
    assert_eq!(
        a.serialise(),
        b.serialise(),
        "order must not leak into the file"
    );
}

#[test]
fn clear_kind_removes_only_that_kind() {
    let mut a = State::default();
    a.set("plan", "1", "x");
    a.set("pace", "prefill", "900");
    a.clear_kind("plan");
    assert!(a.get("plan", "1").is_none());
    assert_eq!(a.get("pace", "prefill"), Some("900"));
}

/// `B2`, on the numbers that row records: keyed by CONTENT HASH the
/// retained set was a hash-sampled slice of ALL history, not a recency
/// window. Measured before the fix -- of 700 observations capped at 200,
/// 62 survivors came from the FIRST two hundred and only 63 from the last.
///
/// `push` makes the key carry a sequence, so the cap drops the
/// genuinely oldest and the window is the last `cap` appended.
#[test]
fn an_ordered_log_keeps_the_newest_not_a_hash_sample() {
    let mut a = State::default();
    for i in 0..700 {
        let line = format!("row {i}");
        let k = content_hash(line.as_bytes());
        a.push(Log::new("obs", 200), &k, line);
    }
    let kept: Vec<usize> = a
        .all("obs")
        .iter()
        .filter_map(|l| l.strip_prefix("row ")?.parse().ok())
        .collect();
    assert_eq!(kept.len(), 200, "the cap holds");
    assert!(
        kept.iter().all(|i| *i >= 500),
        "every retained row is from the last 200 appended; oldest kept \
             was {:?}",
        kept.iter().min()
    );
}

/// The dedup that lets an ordered log avoid a timestamp: re-recording the
/// same observation leaves the file byte-identical, so a state file that
/// changed when nothing changed cannot happen.
#[test]
fn pushing_the_same_entry_twice_changes_nothing() {
    let mut a = State::default();
    a.push(Log::new("obs", 10), "deadbeef", "row");
    let once = a.serialise();
    a.push(Log::new("obs", 10), "deadbeef", "row");
    assert_eq!(a.serialise(), once, "a repeat append is a no-op");
    assert_eq!(a.all("obs").len(), 1);
}

/// A caller that only has the dedup key can still find what it stored,
/// which is what `record_obs_in`'s idempotence rests on.
#[test]
fn an_ordered_entry_is_findable_by_its_dedup_key() {
    let mut a = State::default();
    a.push(Log::new("obs", 10), "cafe", "first");
    a.push(Log::new("obs", 10), "f00d", "second");
    let k = a.find_key("obs", "f00d").unwrap_or_default();
    assert!(k.ends_with("-f00d"), "the dedup key is the suffix: {k}");
    assert_eq!(a.get("obs", &k), Some("second"));
    assert_eq!(a.find_key("obs", "absent"), None);
}

/// A file written before ordered keys existed holds bare content hashes.
/// Appending to it must not restart the sequence at zero -- `ff78...`
/// sorts AFTER `00000000-...`, so reading the last key's sequence found
/// none every time.
///
/// An unsequenced entry is older than every sequenced one, which is true:
/// it was written first, and it is evicted first.
#[test]
fn an_ordered_log_resumes_over_keys_written_before_it_existed() {
    let mut a = State::default();
    for legacy in ["00457224e0ad913c", "ff7833f252862eb6"] {
        a.set("obs", legacy, format!("legacy {legacy}"));
    }
    a.push(Log::new("obs", 10), "aaaa", "new one");
    a.push(Log::new("obs", 10), "bbbb", "new two");
    let keys: Vec<Option<String>> = ["aaaa", "bbbb"]
        .iter()
        .map(|d| a.find_key("obs", d))
        .collect();
    assert_eq!(
        keys,
        vec![
            Some("00000000-aaaa".to_string()),
            Some("00000001-bbbb".to_string())
        ],
        "the sequence advances rather than restarting"
    );
    // And the legacy rows go FIRST when the cap bites.
    a.push(Log::new("obs", 3), "cccc", "new three");
    assert_eq!(a.all("obs").len(), 3);
    assert!(
        a.find_key("obs", "cccc").is_some()
            && a.get("obs", "00457224e0ad913c").is_none(),
        "an unsequenced row is the oldest and is dropped first"
    );
}

#[test]
fn content_hash_tracks_content_only() {
    assert_eq!(content_hash(b"abc"), content_hash(b"abc"));
    assert_ne!(content_hash(b"abc"), content_hash(b"abd"));
}
