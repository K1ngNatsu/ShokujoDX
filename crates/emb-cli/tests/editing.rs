//! The editing helpers of P10 Milestone 1 on all corpus designs: reverse
//! direction and swap sides two times give the same design, and the
//! designs still make stitches after each command.

use emb_cli::{Input, corpus, stitches};
use emb_model::{Design, ObjectId};

fn all_ids(d: &Design) -> Vec<ObjectId> {
    d.objects().iter().map(|o| o.id).collect()
}

fn sews(name: &str, d: &Design) {
    let (s, _) = stitches(&Input::Design(Box::new(d.clone())), None).unwrap();
    assert!(s.stitch_count() > 0 || d.objects().is_empty(), "{name}");
}

#[test]
fn reverse_and_swap_two_times_change_nothing() {
    for name in corpus::NAMES {
        let original = corpus::build(name).unwrap();
        let ids = all_ids(&original);
        let mut d = original.clone();
        let reversed = d.reverse_objects(&ids);
        sews(name, &d);
        assert_eq!(d.reverse_objects(&ids), reversed, "{name}");
        let mut d2 = d.clone();
        let swapped = d2.swap_column_sides(&ids);
        sews(name, &d2);
        assert_eq!(d2.swap_column_sides(&ids), swapped, "{name}");
        // The stitches are the same again. (The paths have the same shape;
        // their commands can be written in another form.)
        let plan = |d: &Design| stitches(&Input::Design(Box::new(d.clone())), None).unwrap().0.records;
        let (a, b) = (plan(&original), plan(&d2));
        assert_eq!(a.len(), b.len(), "{name}");
        for (x, y) in a.iter().zip(&b) {
            assert!(x.kind == y.kind && x.pos.distance(y.pos) < 1e-6, "{name}: {x:?} {y:?}");
        }
    }
}

#[test]
fn close_and_single_stones_keep_the_designs_sewable() {
    for name in corpus::NAMES {
        let mut d = corpus::build(name).unwrap();
        let ids = all_ids(&d);
        let stones = d.stones().len();
        d.close_shapes(&ids, true);
        sews(name, &d);
        // No open part is left.
        let again = d.close_shapes(&ids, false) + d.close_shapes(&ids, true);
        assert_eq!(again, 0, "{name}");
        let mut d = corpus::build(name).unwrap();
        d.to_single_stones(&ids);
        assert_eq!(d.stones().len(), stones, "{name}");
    }
}
