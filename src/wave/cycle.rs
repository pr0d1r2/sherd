//! The CYCLES among a schedule's blocked nodes (`src/wave:V9`): Tarjan's
//! strongly connected components over the blocking edges, one pass.

use super::CodeDep;

/// The cycles among `blocked`, one sorted group per cycle, groups sorted.
///
/// `blocked` also holds nodes that only wait BEHIND a cycle; those are in no
/// group. A group is a strongly connected set of blocking edges -- every
/// member reaches every other, so none can be built first -- and a node
/// that needs itself is a group of one.
#[must_use]
pub fn cycles(deps: &[CodeDep], blocked: &[String]) -> Vec<Vec<String>> {
    let adj = adjacency(deps, blocked);
    let mut t = Tarjan::new(adj.len());
    for v in 0..adj.len() {
        if t.order(v).is_none() {
            t.visit(v, &adj);
        }
    }
    let mut out: Vec<Vec<String>> = t
        .groups
        .into_iter()
        .filter(|g| is_cycle(g, &adj))
        .map(|g| names(&g, blocked))
        .collect();
    out.sort();
    out
}

/// Each blocked node's blocking needs, as positions in `blocked`.
fn adjacency(deps: &[CodeDep], blocked: &[String]) -> Vec<Vec<usize>> {
    let index = |n: &String| blocked.iter().position(|b| b == n);
    blocked
        .iter()
        .map(|b| {
            deps.iter()
                .find(|d| &d.node == b)
                .map(|d| d.needs.iter().filter_map(index).collect())
                .unwrap_or_default()
        })
        .collect()
}

/// More than one member, or one that needs itself.
fn is_cycle(g: &[usize], adj: &[Vec<usize>]) -> bool {
    g.len() > 1
        || g.iter()
            .any(|v| adj.get(*v).is_some_and(|ws| ws.contains(v)))
}

/// A group's labels, sorted.
fn names(g: &[usize], blocked: &[String]) -> Vec<String> {
    let mut out: Vec<String> =
        g.iter().filter_map(|v| blocked.get(*v).cloned()).collect();
    out.sort();
    out
}

/// One node's Tarjan state: visit order, lowest order it reaches, whether
/// it is on the stack.
#[derive(Clone, Copy, Default)]
struct Mark {
    order: Option<usize>,
    low: usize,
    on_stack: bool,
}

/// Tarjan's bookkeeping.
struct Tarjan {
    next: usize,
    marks: Vec<Mark>,
    stack: Vec<usize>,
    groups: Vec<Vec<usize>>,
}

impl Tarjan {
    fn new(n: usize) -> Self {
        Self {
            next: 0,
            marks: vec![Mark::default(); n],
            stack: Vec::new(),
            groups: Vec::new(),
        }
    }

    fn mark(&self, v: usize) -> Mark {
        self.marks.get(v).copied().unwrap_or_default()
    }

    fn order(&self, v: usize) -> Option<usize> {
        self.mark(v).order
    }

    fn set(&mut self, v: usize, f: impl FnOnce(&mut Mark)) {
        if let Some(m) = self.marks.get_mut(v) {
            f(m);
        }
    }

    fn visit(&mut self, v: usize, adj: &[Vec<usize>]) {
        let n = self.next;
        self.set(v, |m| {
            *m = Mark {
                order: Some(n),
                low: n,
                on_stack: true,
            }
        });
        self.next = n.saturating_add(1);
        self.stack.push(v);
        for &w in adj.get(v).map_or(&[][..], Vec::as_slice) {
            let reach = match self.mark(w) {
                Mark { order: None, .. } => {
                    self.visit(w, adj);
                    self.mark(w).low
                }
                Mark {
                    order: Some(o),
                    on_stack: true,
                    ..
                } => o,
                Mark { .. } => continue,
            };
            self.set(v, |m| m.low = m.low.min(reach));
        }
        let m = self.mark(v);
        if Some(m.low) == m.order {
            self.close(v);
        }
    }

    /// Pop the stack down to `v`, the root of a finished component.
    fn close(&mut self, v: usize) {
        let mut group = Vec::new();
        while let Some(w) = self.stack.pop() {
            self.set(w, |m| m.on_stack = false);
            group.push(w);
            if w == v {
                break;
            }
        }
        self.groups.push(group);
    }
}
