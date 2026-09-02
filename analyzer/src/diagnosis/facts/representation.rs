//! 本文件仅在全部非空 X 文本可解析且 JSON AST 严格相等时附加等价表示事实。

use actrail_kv_artifacts::{DefectFact, DefectFactKind, OptimizationInsight};

use super::super::episode::variant::LocatedVariant;

pub(super) struct RepresentationFacts {
    pub facts: Vec<DefectFact>,
    pub insights: Vec<OptimizationInsight>,
    pub certainty: f64,
}

pub(super) fn analyze(variants: &[LocatedVariant]) -> RepresentationFacts {
    let texts: Vec<_> = variants
        .iter()
        .filter_map(|variant| (!variant.text.is_empty()).then_some(&variant.text))
        .collect();
    let parsed: Option<Vec<serde_json::Value>> = texts
        .iter()
        .map(|text| serde_json::from_str(text).ok())
        .collect();
    let equivalent = parsed.as_ref().is_some_and(|values| {
        values.len() >= 2
            && values.iter().all(|value| value == &values[0])
            && texts.iter().any(|text| text.as_str() != texts[0].as_str())
    });
    if !equivalent {
        return RepresentationFacts {
            facts: vec![],
            insights: vec![],
            certainty: 1.0,
        };
    }
    RepresentationFacts {
        facts: vec![DefectFact {
            kind: DefectFactKind::StructuredDataEquivalent,
            detail: Some("模型可见 JSON 的语法树严格相等，但原始文本表示不同".into()),
        }],
        insights: vec![OptimizationInsight {
            summary: "使用确定的模型可见序列化格式".into(),
            detail: Some("固定对象键序、空白、换行与转义策略".into()),
        }],
        certainty: 1.0,
    }
}
