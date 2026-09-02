//! Bounded deterministic dynamic programming aligns observable units without tokenizer access.

use crate::model::projection::CacheUnit;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnitAlignment {
    pub medoid_to_member: Vec<Option<usize>>,
    pub unmatched_member_units: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlignmentError {
    CellBudgetExceeded { required: usize, limit: usize },
}

#[derive(Clone, Debug)]
pub struct SequenceAligner {
    max_cells: usize,
}

impl SequenceAligner {
    pub fn new(max_cells: usize) -> Self {
        Self { max_cells }
    }

    pub fn align(
        &self,
        medoid: &[CacheUnit],
        member: &[CacheUnit],
    ) -> Result<UnitAlignment, AlignmentError> {
        let required = medoid
            .len()
            .saturating_add(1)
            .saturating_mul(member.len().saturating_add(1));
        if required > self.max_cells {
            return Err(AlignmentError::CellBudgetExceeded {
                required,
                limit: self.max_cells,
            });
        }
        let columns = member.len() + 1;
        let mut scores = vec![0i32; (medoid.len() + 1) * columns];
        for i in 1..=medoid.len() {
            scores[i * columns] = -(i as i32);
        }
        for (j, score) in scores.iter_mut().take(member.len() + 1).enumerate().skip(1) {
            *score = -(j as i32);
        }
        for i in 1..=medoid.len() {
            for j in 1..=member.len() {
                let same_key = medoid[i - 1].alignment_key == member[j - 1].alignment_key
                    && medoid[i - 1].kind == member[j - 1].kind;
                let content_bonus = i32::from(medoid[i - 1].content == member[j - 1].content);
                let diagonal = scores[(i - 1) * columns + j - 1]
                    + if same_key { 2 + content_bonus } else { -2 };
                let delete = scores[(i - 1) * columns + j] - 1;
                let insert = scores[i * columns + j - 1] - 1;
                scores[i * columns + j] = diagonal.max(delete).max(insert);
            }
        }
        let mut mapping = vec![None; medoid.len()];
        let mut matched_member = vec![false; member.len()];
        let (mut i, mut j) = (medoid.len(), member.len());
        while i > 0 || j > 0 {
            if i > 0 && j > 0 {
                let same_key = medoid[i - 1].alignment_key == member[j - 1].alignment_key
                    && medoid[i - 1].kind == member[j - 1].kind;
                let content_bonus = i32::from(medoid[i - 1].content == member[j - 1].content);
                let diagonal = scores[(i - 1) * columns + j - 1]
                    + if same_key { 2 + content_bonus } else { -2 };
                if same_key && scores[i * columns + j] == diagonal {
                    mapping[i - 1] = Some(j - 1);
                    matched_member[j - 1] = true;
                    i -= 1;
                    j -= 1;
                    continue;
                }
            }
            if i > 0 && scores[i * columns + j] == scores[(i - 1) * columns + j] - 1 {
                i -= 1;
            } else if j > 0 {
                j -= 1;
            } else {
                i -= 1;
            }
        }
        Ok(UnitAlignment {
            medoid_to_member: mapping,
            unmatched_member_units: matched_member
                .into_iter()
                .enumerate()
                .filter_map(|(index, matched)| (!matched).then_some(index))
                .collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::projection::{CacheUnitKind, SourceLocation};

    fn unit(key: &str, value: &str) -> CacheUnit {
        CacheUnit {
            kind: CacheUnitKind::VisibleText,
            alignment_key: key.into(),
            content: value.into(),
            source: SourceLocation {
                json_path: "$".into(),
                utf8_bytes: Some(0..value.len()),
                message_index: None,
                role: None,
            },
            tool_identity: None,
        }
    }

    #[test]
    fn aligns_inserted_history_instead_of_positional_zipping() {
        let medoid = vec![unit("system", "rules"), unit("user", "question")];
        let member = vec![
            unit("system", "rules"),
            unit("assistant", "old"),
            unit("user", "question"),
        ];
        let aligned = SequenceAligner::new(100).align(&medoid, &member).unwrap();
        assert_eq!(aligned.medoid_to_member, [Some(0), Some(2)]);
        assert_eq!(aligned.unmatched_member_units, [1]);
    }

    #[test]
    fn cell_limit_is_inclusive() {
        let units = vec![unit("x", "x")];
        assert!(SequenceAligner::new(4).align(&units, &units).is_ok());
        assert!(SequenceAligner::new(3).align(&units, &units).is_err());
    }
}
