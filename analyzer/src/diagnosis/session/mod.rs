//! 本模块分析显式 Session 时间线上相邻请求的可观察前缀延续。

mod aggregate;
mod model;
mod prefix;
mod timeline;

#[cfg(test)]
mod tests;

pub use model::SessionObservation;

use crate::model::projection::CacheSequence;
use actrail_kv_artifacts::SessionAnalysis;

pub fn analyze_session(
    observations: Vec<SessionObservation>,
    sequences: &[CacheSequence],
) -> SessionAnalysis {
    let session_record_count = observations.len();
    let timelines = timeline::build(observations, sequences);
    let history_sites = aggregate::history_sites(&timelines);
    SessionAnalysis {
        session_record_count,
        timelines,
        history_sites,
    }
}
