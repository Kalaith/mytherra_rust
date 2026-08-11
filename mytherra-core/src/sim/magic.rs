//! Per-tick magic research (GDD 5.6): every path advances on the world's arcane
//! affinity, matures through thresholds, and — once emerging/known — passively
//! reshapes every region; a fully Known path reaches living heroes too, letting
//! legend grow in attuned lands. Deterministic: no RNG.

use crate::data::strings::ChronicleText;
use crate::data::{
    fill, ArtifactFocus, Culture, HeroRole, MagicBalance, MagicStat, RegionBalance, ResourceStatus,
    ResourceType,
};
use crate::world::{
    Artifact, Chronicle, EventKind, Hero, Landmark, MagicPath, MagicState, Region, ResourceNode,
};

/// Advance every research path by one tick and apply mature paths' effects.
#[allow(clippy::too_many_arguments)]
pub fn tick_magic(
    paths: &mut [MagicPath],
    regions: &mut [Region],
    heroes: &mut [Hero],
    artifacts: &[Artifact],
    landmarks: &[Landmark],
    resource_nodes: &[ResourceNode],
    balance: &MagicBalance,
    region_balance: &RegionBalance,
    chronicle: &mut Chronicle,
    text: &ChronicleText,
    year: u32,
) {
    let avg_magic = average_magic(regions);

    // Evidence of the arcane builds fastest where minds study it: living scholars
    // and mages hasten every path's maturation, so a learned age masters magic
    // sooner than an unlettered one (GDD 5.6 <-> 5.4). Counted once, applied to
    // all paths.
    let learned = heroes
        .iter()
        .filter(|h| h.is_alive && matches!(h.role, HeroRole::Scholar | HeroRole::Mage))
        .count();
    let scholar_evidence = learned as f32 * balance.evidence_per_scholar;

    // A relic of knowledge is itself a font of understanding: every Knowledge-
    // focus artifact hastens research by its power, so the Artifacts tool feeds
    // the Magic tool (GDD 5.6). Distinct from a relic's affinity nudge — this is
    // insight into the arcane (evidence), not the ambient magic of the land.
    let relic_evidence = artifacts
        .iter()
        .filter(|a| a.focus == ArtifactFocus::Knowledge)
        .map(|a| a.power as f32 * balance.evidence_per_knowledge_relic)
        .sum::<f32>();

    // The great libraries and arcane towers are the houses of the world's
    // learning: every scholarly or mystical wonder hastens research by its
    // cultural weight — its influence times its storied stature — so a land of
    // such wonders masters magic sooner, an ancient one more than a new (GDD 5.6
    // <-> 5.2).
    let landmark_evidence = landmarks
        .iter()
        .filter(|l| matches!(l.culture, Culture::Scholarly | Culture::Mystical))
        .map(|l| l.influence * l.stature * balance.evidence_per_learned_landmark)
        .sum::<f32>();

    // The world's own wellsprings of magic are raw material for study: every
    // producing Manaspring hastens research, so the natural arcana of the land
    // masters magic alongside its scholars and its towers (GDD 5.6 <-> 5.3). A
    // spring run dry offers nothing.
    let manaspring_evidence = resource_nodes
        .iter()
        .filter(|n| {
            n.resource_type == ResourceType::Manaspring && n.status != ResourceStatus::Depleted
        })
        .count() as f32
        * balance.evidence_per_manaspring;

    // The world's practical learning readies it for the arcane: a civilization
    // deep in medicine, engineering, and husbandry grasps magic sooner than a
    // benighted one, so the average lore of the realms — above the common measure
    // every land begins with — hastens every path (GDD 5.6 <-> 5.4). This closes
    // the knowledge cycle begun in tick_lore, where a Known path raises the lore
    // ceiling: risen lore now hastens the next path in turn. Read once, applied to
    // all. Deterministic — a mean of world state, no roll.
    let avg_lore = if regions.is_empty() {
        0.0
    } else {
        regions.iter().map(|r| r.lore).sum::<f32>() / regions.len() as f32
    };
    let lore_evidence =
        (avg_lore - balance.evidence_lore_floor).max(0.0) * balance.evidence_per_lore;

    for path in paths.iter_mut() {
        path.progress =
            (path.progress + balance.progress_per_tick + avg_magic * balance.magic_affinity_coeff)
                .min(balance.stat_cap);
        path.evidence = (path.evidence
            + balance.evidence_per_tick
            + scholar_evidence
            + relic_evidence
            + landmark_evidence
            + manaspring_evidence
            + lore_evidence)
            .min(balance.stat_cap);
        path.recompute_state(balance);

        if path.state == MagicState::Known && !path.announced_known {
            path.announced_known = true;
            chronicle.push(
                year,
                EventKind::System,
                fill(&text.magic_known, &[("path", path.name.clone())]),
            );
        }

        let scale = path.effect_scale(balance);
        if scale > 0.0 {
            let amount = path.effect_per_tick * scale;
            for region in regions.iter_mut() {
                // Magic bites deepest where the arcane runs strong.
                let attunement =
                    balance.affinity_base + region.magic_affinity * balance.affinity_coeff;
                let (dp, dc, dd, dm) = stat_deltas(path.effect_stat, amount * attunement);
                region.apply_deltas(dp, dc, dd, dm, region_balance);
            }
        }
    }

    // Magic reaches living things, not just the land: each Known path lets legend
    // grow, granting living heroes renown scaled by their region's attunement
    // (GDD 5.6 — the deepest of the seven tools).
    let known = paths
        .iter()
        .filter(|p| p.state == MagicState::Known)
        .count();
    if known > 0 && balance.known_renown_per_tick > 0.0 {
        let base = known as f32 * balance.known_renown_per_tick;
        for hero in heroes.iter_mut().filter(|h| h.is_alive) {
            let attunement = regions
                .iter()
                .find(|r| r.id == hero.region_id)
                .map(|r| balance.affinity_base + r.magic_affinity * balance.affinity_coeff)
                .unwrap_or(balance.affinity_base);
            hero.renown += base * attunement;
        }
    }
}

fn average_magic(regions: &[Region]) -> f32 {
    if regions.is_empty() {
        return 0.0;
    }
    regions.iter().map(|r| r.magic_affinity).sum::<f32>() / regions.len() as f32
}

/// Map a magic stat + amount onto (prosperity, chaos, danger, magic) deltas.
fn stat_deltas(stat: MagicStat, amount: f32) -> (f32, f32, f32, f32) {
    match stat {
        MagicStat::Prosperity => (amount, 0.0, 0.0, 0.0),
        MagicStat::Chaos => (0.0, amount, 0.0, 0.0),
        MagicStat::Danger => (0.0, 0.0, amount, 0.0),
        MagicStat::Magic => (0.0, 0.0, 0.0, amount),
    }
}

#[cfg(test)]
mod tests;
