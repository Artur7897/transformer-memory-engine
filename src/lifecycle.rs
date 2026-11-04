//! TME lifecycle: operational state gates maintenance; no wall-clock scheduler.
//! Safety-critical control and LLM process management remain owned by Soul.
use crate::{maintenance::{MaintenanceAssessment, MaintenanceMode, MaintenanceSignal}, Memory};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum TmePhase { Active, Standby, Sleep }
impl Default for TmePhase { fn default() -> Self { Self::Active } }

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct TmeLifecycle {
    #[serde(default)] pub phase: TmePhase,
    #[serde(default)] pub pending_maintenance: bool,
    #[serde(default)] pub completed_sleep_passes: u64,
}

impl Memory {
    pub fn lifecycle_request_maintenance(&mut self) {
        self.lifecycle.pending_maintenance=true;
    }
    pub fn lifecycle_enter_standby(&mut self, active_tasks: usize) -> Result<(),String> {
        if active_tasks != 0 { return Err("Cannot enter standby with active tasks".into()); }
        if self.lifecycle.phase != TmePhase::Active { return Err("Expected active phase".into()); }
        self.lifecycle.phase=TmePhase::Standby;
        Ok(())
    }
    pub fn lifecycle_begin_sleep(&mut self, llm_inactive: bool) -> Result<(),String> {
        if self.lifecycle.phase != TmePhase::Standby || !self.lifecycle.pending_maintenance {
            return Err("Sleep requires standby and pending maintenance".into());
        }
        if !llm_inactive { return Err("LLM must be inactive for this sleep profile".into()); }
        self.lifecycle.phase=TmePhase::Sleep;
        Ok(())
    }
    /// Read-only assessment during SLEEP. Semantic tasks are not executed here.
    pub fn lifecycle_sleep_assess(&self, budget:usize) -> Result<MaintenanceAssessment,String> {
        if self.lifecycle.phase != TmePhase::Sleep {return Err("Not in sleep".into());}
        self.maintenance_assess(MaintenanceMode::Sleep,MaintenanceSignal::Routine,budget)
    }
    /// Only mark the inspection as finished. Real mutations need separate,
    /// explicit validated commits; this does not silently consolidate tokens.
    pub fn lifecycle_finish_sleep(&mut self) -> Result<(),String> {
        if self.lifecycle.phase != TmePhase::Sleep { return Err("Not in sleep".into()); }
        self.lifecycle.completed_sleep_passes=self.lifecycle.completed_sleep_passes
            .checked_add(1).ok_or("Sleep pass counter overflow")?;
        // A completed inspection is not the same as a clean memory state.
        // Unresolved findings must remain scheduled for later review.
        let unresolved = self.maintenance_assess(
            MaintenanceMode::Sleep, MaintenanceSignal::Routine, usize::MAX,
        )?.findings_total > 0;
        self.lifecycle.pending_maintenance=unresolved;
        self.lifecycle.phase=TmePhase::Standby;
        Ok(())
    }
    pub fn lifecycle_wake(&mut self) {
        self.lifecycle.phase=TmePhase::Active;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sleep_is_state_driven_and_llm_inactive() {
        let mut m=Memory::new();
        assert!(m.lifecycle_begin_sleep(true).is_err());
        m.lifecycle_request_maintenance();
        assert!(m.lifecycle_enter_standby(1).is_err());
        m.lifecycle_enter_standby(0).unwrap();
        assert!(m.lifecycle_begin_sleep(false).is_err());
        m.lifecycle_begin_sleep(true).unwrap();
        assert!(m.lifecycle_sleep_assess(0).is_err());
        m.lifecycle_sleep_assess(20).unwrap();
        m.lifecycle_finish_sleep().unwrap();
        assert_eq!(m.lifecycle.completed_sleep_passes,1);
        m.lifecycle_wake();
        assert_eq!(m.lifecycle.phase,TmePhase::Active);
    }
}

#[cfg(test)]
mod stage6_tests {
    use super::*;
    use crate::StateToken;
    #[test]
    fn sleep_does_not_silently_discard_unresolved_findings() {
        let mut m=Memory::new();
        m.insert(StateToken{id:"a".into(),vector:vec![1.,0.]}).unwrap();
        m.record_token_operation("a").unwrap();
        m.lifecycle_request_maintenance();
        m.lifecycle_enter_standby(0).unwrap();
        m.lifecycle_begin_sleep(true).unwrap();
        m.lifecycle_sleep_assess(10).unwrap();
        m.lifecycle_finish_sleep().unwrap();
        assert!(m.lifecycle.pending_maintenance);
        m.lifecycle_begin_sleep(true).unwrap();
        m.maintenance_review_operations(MaintenanceMode::Sleep,10).unwrap();
        m.lifecycle_finish_sleep().unwrap();
        assert!(!m.lifecycle.pending_maintenance);
    }
}
