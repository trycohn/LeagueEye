use crate::lcu;

pub const MATCH_ACTIVE_PHASE: &str = "InProgress";

pub fn allows_overlay_for_phase(phase: &str) -> bool {
    phase == MATCH_ACTIVE_PHASE
}

pub fn current_overlay_eligibility() -> bool {
    let Some(creds) = lcu::detect_lcu_credentials() else {
        return false;
    };

    let Ok(phase) = lcu::get_gameflow_phase(&creds) else {
        return false;
    };

    allows_overlay_for_phase(&phase)
}

#[cfg(test)]
mod tests {
    use super::allows_overlay_for_phase;

    #[test]
    fn overlays_are_only_allowed_during_in_progress() {
        assert!(allows_overlay_for_phase("InProgress"));

        for phase in [
            "",
            "ChampSelect",
            "GameStart",
            "Reconnect",
            "EndOfGame",
            "WaitingForStats",
            "Lobby",
        ] {
            assert!(!allows_overlay_for_phase(phase), "phase {phase} should be blocked");
        }
    }
}
