//! Runtime pantheon state (GDD 5.6): four deities whose pressure rises and
//! falls, pressing their domain upon the world in tiers, connected in a fixed
//! ally/rival diamond so appeasing or challenging one ripples to the others.

use crate::data::{DeitySeed, PantheonBalance, PantheonStat};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PantheonDeity {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub ally_id: String,
    pub rival_id: String,
    pub effect_stat: PantheonStat,
    pub effect_amount: f32,
    /// 0-100 agitation; higher pressure presses the domain harder.
    pub pressure: f32,
    /// Years until this deity may be appeased/challenged again.
    pub cooldown: i32,
}

impl PantheonDeity {
    pub fn from_seed(seed: &DeitySeed) -> Self {
        Self {
            id: seed.id.clone(),
            name: seed.name.clone(),
            domain: seed.domain.clone(),
            ally_id: seed.ally_id.clone(),
            rival_id: seed.rival_id.clone(),
            effect_stat: seed.effect_stat,
            effect_amount: seed.effect_amount,
            pressure: seed.start_pressure.clamp(0.0, 100.0),
            cooldown: 0,
        }
    }

    /// The tier index the deity's pressure has reached (0 = below the first
    /// tier / dormant, 1..=tiers.len() as pressure climbs).
    pub fn tier(&self, balance: &PantheonBalance) -> usize {
        let mut tier = 0;
        for (i, threshold) in balance.tiers.iter().enumerate() {
            if self.pressure >= *threshold {
                tier = i + 1;
            }
        }
        tier
    }

    /// Effect multiplier for the current tier (0 while dormant).
    pub fn tier_multiplier(&self, balance: &PantheonBalance) -> f32 {
        let tier = self.tier(balance);
        if tier == 0 {
            0.0
        } else {
            balance.tier_mults.get(tier - 1).copied().unwrap_or(0.0)
        }
    }
}

/// Add a pressure delta to the deity with the given id, clamped to 0-100.
pub fn adjust_pressure(deities: &mut [PantheonDeity], id: &str, delta: f32) {
    if let Some(deity) = deities.iter_mut().find(|d| d.id == id) {
        deity.pressure = (deity.pressure + delta).clamp(0.0, 100.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balance() -> PantheonBalance {
        crate::data::GameData::load().unwrap().balance.pantheon
    }

    fn deity(pressure: f32) -> PantheonDeity {
        PantheonDeity::from_seed(&DeitySeed {
            id: "d".to_owned(),
            name: "D".to_owned(),
            domain: "Test".to_owned(),
            ally_id: "a".to_owned(),
            rival_id: "r".to_owned(),
            effect_stat: PantheonStat::Prosperity,
            effect_amount: 0.3,
            start_pressure: pressure,
        })
    }

    #[test]
    fn tier_rises_with_pressure() {
        let b = balance();
        assert_eq!(deity(10.0).tier(&b), 0);
        assert!(deity(80.0).tier(&b) > deity(30.0).tier(&b));
        assert!(deity(90.0).tier_multiplier(&b) > deity(30.0).tier_multiplier(&b));
    }

    #[test]
    fn adjust_pressure_clamps() {
        let mut deities = vec![deity(95.0)];
        deities[0].id = "x".to_owned();
        adjust_pressure(&mut deities, "x", 50.0);
        assert!(deities[0].pressure <= 100.0);
    }
}
