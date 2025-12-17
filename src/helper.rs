use cidre::cg;

pub fn fix_cursor() {
    let state_id = cg::EventSrcStateId::CombinedSession;
    let mut event_source_ref = cg::EventSrc::with_state(state_id);
    if let Some(ref mut retained) = event_source_ref {
        cg::EventSrc::set_local_events_suppression_interval(retained, 0.0);
    }
}