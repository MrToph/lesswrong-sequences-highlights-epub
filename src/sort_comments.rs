use std::collections::HashMap;

use lesswrong_api::Comment;

fn sort_children_recursive(
    parent: Option<String>,
    results: &mut Vec<Comment>,
    max_comments: &usize,
    comments: &HashMap<String, Comment>,
) {
    // get all direct children and sort them by score descending
    let mut children = comments
        .values()
        .filter(|c| c.parent_comment_id == parent)
        .cloned()
        .collect::<Vec<_>>();
    children.sort_by(|a, b| b.base_score.partial_cmp(&a.base_score).unwrap());

    // pick up child that is being processed, then recurse (depth-first)
    for child in children.drain(..) {
        if results.len() >= *max_comments {
            return;
        }
        let child_id = child.id.clone();
        results.push(child);
        sort_children_recursive(Some(child_id), results, max_comments, comments);
    }
}

pub fn sort_comments_by_score_depth_first(
    comments: &HashMap<String, Comment>,
    max_comments: usize,
) -> Vec<Comment> {
    let max_comments = std::cmp::min(max_comments, comments.len());
    let mut results = Vec::with_capacity(max_comments);
    sort_children_recursive(None, &mut results, &max_comments, comments);
    results
}

#[cfg(test)]
mod tests {
    use super::*;

    fn do_vecs_match<T: PartialEq>(a: &[T], b: &[T]) -> bool {
        let matching = a.iter().zip(b.iter()).filter(|&(a, b)| a == b).count();
        matching == a.len() && matching == b.len()
    }

    #[test]
    fn test_sort_comments_by_score_depth_first() {
        let mut comments = HashMap::new();
        comments.insert(
            "a".into(),
            Comment {
                id: "a".into(),
                parent_comment_id: None,
                base_score: 1.0,
                ..Default::default()
            },
        );
        comments.insert(
            "b".into(),
            Comment {
                id: "b".into(),
                parent_comment_id: None,
                base_score: 2.0,
                ..Default::default()
            },
        );
        comments.insert(
            "aa".into(),
            Comment {
                id: "aa".into(),
                parent_comment_id: Some("a".into()),
                base_score: 100.0,
                ..Default::default()
            },
        );
        comments.insert(
            "ab".into(),
            Comment {
                id: "ab".into(),
                parent_comment_id: Some("a".into()),
                base_score: 101.0,
                ..Default::default()
            },
        );
        comments.insert(
            "ba".into(),
            Comment {
                id: "ba".into(),
                parent_comment_id: Some("b".into()),
                base_score: 10.0,
                ..Default::default()
            },
        );
        comments.insert(
            "bb".into(),
            Comment {
                id: "bb".into(),
                parent_comment_id: Some("b".into()),
                base_score: 11.0,
                ..Default::default()
            },
        );

        // should sort b subtree before a subtree, then bb before ba, etc.
        let mut sorted = sort_comments_by_score_depth_first(&comments, 5);
        assert!(do_vecs_match(
            &sorted.drain(..).map(|c| c.id).collect::<Vec<String>>(),
            &[
                "b".into(),
                "bb".into(),
                "ba".into(),
                "a".into(),
                "ab".into()
            ],
        ));
    }
}
