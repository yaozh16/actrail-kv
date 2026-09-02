//! 本文件识别同一逻辑 region 的内容变化与少数固定版本，不依赖业务值形状。

use std::collections::BTreeSet;

use actrail_kv_artifacts::{DefectFact, DefectFactKind, OptimizationInsight};

use super::super::episode::variant::LocatedVariant;

pub(super) struct ContentFacts {
    pub present: bool,
    pub facts: Vec<DefectFact>,
    pub insights: Vec<OptimizationInsight>,
    pub certainty: f64,
}

pub(super) fn analyze(variants: &[LocatedVariant]) -> ContentFacts {
    let nonempty: Vec<_> = variants
        .iter()
        .filter(|variant| !variant.parts.is_empty())
        .collect();
    let signatures: BTreeSet<_> = nonempty.iter().map(|item| &item.fingerprint).collect();
    let same_shape = nonempty
        .first()
        .is_some_and(|first| nonempty.iter().all(|item| item.shape == first.shape));
    let present = same_shape && signatures.len() > 1;
    if !present {
        return ContentFacts {
            present: false,
            facts: vec![],
            insights: vec![],
            certainty: 1.0,
        };
    }
    let fixed = signatures.len() <= 3 && signatures.len() < nonempty.len();
    ContentFacts {
        present: true,
        facts: vec![DefectFact {
            kind: if fixed {
                DefectFactKind::FixedVariants
            } else {
                DefectFactKind::ContentVariation
            },
            detail: Some(format!("观察到 {} 个内容变体", signatures.len())),
        }],
        insights: vec![OptimizationInsight {
            summary: if fixed {
                "统一稳定内容的配置或模板版本".into()
            } else {
                "评估将易变内容后移到稳定上下文之后".into()
            },
            detail: Some("同一逻辑位置的变化过早阻断了后续稳定前缀".into()),
        }],
        certainty: 1.0,
    }
}
