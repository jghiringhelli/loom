# Invellum BIOISO Fitness Oracle

**Purpose:** This document is the canonical fitness specification for BIOISO evolution on Invellum —
a B2C social platform for entrepreneurs. It encodes professional marketing and product frameworks
as measurable signal definitions, fitness functions, and mutation spaces. Every BIOISO domain for
Invellum derives its `metric_bounds`, signal feeds, and `telos` from a named section below.

**Audience:** BIOISO colony (stateless reader + MeiosisEngine), product team, T5 genome reviewer.

---

## 1. Theoretical Foundation

The fitness oracle is grounded in five professional frameworks, chosen because each makes
measurable predictions about what causes users to stay, engage, and pay.

### 1.1 AARRR Funnel (Dave McClure, 2007)

Five stages, each with a conversion gate. A mutation is positive if it improves a gate without
degrading an earlier gate. Degradation of an earlier gate disqualifies a mutation regardless of
downstream gains.

| Stage | Definition | Gate event (Railway log) | Target rate |
|-------|-----------|--------------------------|-------------|
| Acquisition | First visit | `page_view page=landing` | baseline |
| Activation | Experienced core value | `first_connection` OR `first_post` within 24h of signup | ≥ 35% |
| Retention | Returns habitually | WAU / MAU ≥ 0.35 (sticky ratio) | ≥ 0.35 |
| Revenue | Pays | `subscription_upgraded` | ≥ 8% of activated |
| Referral | Brings others | `invite_sent` that results in `signup_completed` | ≥ 15% of active |

**Activation threshold (35%):** Industry benchmark for social/professional networks in pre-PMF
stage (Croll & Yoskovitz, *Lean Analytics*, 2013). Invellum-specific aha moment is defined as
the first time a user both follows someone AND posts something, because entrepreneurs derive value
from audience and discourse simultaneously.

### 1.2 Hooked Model (Nir Eyal, 2014)

Four-phase habit loop. BIOISO treats loop health as a compound signal: a well-running loop
produces increasing Investment scores over cohort weeks. Degraded loops show declining session
starts within 72h of last reward.

| Phase | What it measures | Signal proxy |
|-------|-----------------|--------------|
| Trigger | External push → app open | notification → `session_start` within 2h |
| Action | Minimal friction task | `page_view count` per session ≥ 3 |
| Variable Reward | Discovery of something new | feed items marked `is_new_connection_post: true` in view |
| Investment | User adds value | `post_created` OR `comment_written` OR `connection_invited` per week |

Loop health score: `(trigger_open_rate × 0.20) + (action_depth × 0.25) + (reward_diversity × 0.25) + (investment_rate × 0.30)`

Loop health ≥ 0.60 is the T2 promotion gate for the FeedEngagement domain.

### 1.3 BJ Fogg Behavior Model (2009)

`Behavior = Motivation × Ability × Prompt`

Practical implication: mutations that reduce Ability (friction reduction) have higher ROI than
mutations that increase Motivation (copy changes) when Ability is the bottleneck. Identify the
bottleneck by measuring where drop-off occurs in the funnel:

- Drop-off at signup → Ability bottleneck → simplify form, reduce fields
- Drop-off at first connection → Motivation bottleneck → improve suggestion quality/copy
- Drop-off at first post → Ability bottleneck OR Motivation → A/B prompt vs. blank composer

BIOISO mutation priority order: Ability fixes first (remove friction), then Prompt improvements
(notification/CTA copy), then Motivation enhancements (social proof, authority signals).

### 1.4 SaaS Retention Economics (David Skok, forEntrepreneurs.com)

For a B2C social platform, revenue is a lagging indicator. The leading indicators that predict
LTV are:

- **Day 1 retention** — if < 60%, product has a viscosity problem (too hard to understand)
- **Day 7 retention** — if < 30%, product has a relevance problem (not solving the job)
- **Day 30 retention** — if < 15%, product has a habit problem (no loop formed)
- **Monthly churn rate** — target < 5% for early-stage B2C; > 8% signals structural mismatch

These are the four inviolable bounds in the RetentionGuard domain. Any mutation that improves
a downstream metric while degrading a retention threshold is **rejected by rule**, regardless
of T-tier.

### 1.5 Cialdini Influence Principles (Robert Cialdini, *Influence*, 1984)

Six principles encoded as mutation archetypes that BIOISO may propose for UI/copy changes:

| Principle | Mutation archetype | Signal that confirms it worked |
|-----------|------------------|-------------------------------|
| Social Proof | "X other entrepreneurs from your industry joined this week" | signup_rate ↑ |
| Authority | Display advisor credentials on landing page | upgrade_rate ↑ |
| Reciprocity | Free feature unlock on first post | post_created rate ↑ |
| Commitment | Onboarding goal-setting step | Day 7 retention ↑ |
| Scarcity | "Early member" badge fading after N cohort | invite_sent rate ↑ |
| Liking | Show mutual connections on profile | connection_rate ↑ |

Each archetype maps to a specific page/component mutation and a single confirmation signal.
BIOISO must not apply more than one Cialdini archetype per page per generation — combinatorial
effect is uncharted and violates the isolate-one-variable principle.

### 1.6 Jobs-to-be-Done (Clayton Christensen, *Competing Against Luck*, 2016)

Entrepreneurs hire Invellum for three simultaneous jobs:
1. **Functional:** Find partners, customers, or capital faster than cold outreach
2. **Emotional:** Feel connected to peers who understand the startup struggle
3. **Social:** Signal credibility to the market ("I'm in serious company")

BIOISO mutations must not undermine the Social job (credibility signal) even when optimizing for
Functional or Emotional metrics. This is encoded as a soft constraint: the `@credibility_safe`
annotation on a mutation proposal means it has been reviewed for Social job impact. Mutations
without this annotation are held at T2 pending manual review.

---

## 2. Signal Catalog (Railway Log Patterns)

Invellum emits structured JSON logs via Railway. BIOISO reads these as signal feeds.
Each signal is a rolling 7-day average unless noted as instantaneous.

### 2.1 Acquisition Signals

```
{ "event": "page_view", "page": "landing", "source": "organic|paid|referral|direct" }
{ "event": "signup_started" }
{ "event": "signup_completed", "cohort_week": N, "source": "...", "industry": "..." }
{ "event": "signup_abandoned", "step": "email|profile|goal" }
```

Signal: `acquisition_rate` = `signup_completed / page_view[page=landing]` (7d rolling)
Signal: `form_abandonment_step` = argmax of `signup_abandoned.step` distribution

### 2.2 Activation Signals

```
{ "event": "first_connection", "user_id": "...", "hours_since_signup": N }
{ "event": "first_post", "user_id": "...", "hours_since_signup": N }
{ "event": "aha_moment", "user_id": "...", "hours_since_signup": N }  -- both above within 24h
{ "event": "onboarding_step_completed", "step": "profile|goal|discover|connect|post" }
{ "event": "onboarding_abandoned", "step": "..." }
```

Signal: `activation_rate` = `aha_moment / signup_completed` (7d rolling)
Signal: `activation_time_p50` = median hours_since_signup at aha_moment
Signal: `onboarding_drop_step` = argmax of `onboarding_abandoned.step` distribution

### 2.3 Engagement Signals

```
{ "event": "session_start", "user_id": "...", "trigger": "notification|direct|email" }
{ "event": "session_end", "user_id": "...", "duration_s": N, "page_views": N }
{ "event": "feed_item_viewed", "item_id": "...", "is_connection_post": bool, "dwell_ms": N }
{ "event": "feed_item_clicked", "item_id": "...", "action": "like|comment|share|connect" }
{ "event": "notification_opened", "type": "connection_request|comment|like|weekly_digest" }
```

Signal: `session_depth` = avg `page_views` per session (7d rolling)
Signal: `feed_engagement_rate` = `feed_item_clicked / feed_item_viewed` (7d rolling)
Signal: `notification_open_rate` = `notification_opened / notification_sent` (7d rolling)
Signal: `loop_health` = composite (§1.2 formula applied to above)

### 2.4 Retention Signals

```
{ "event": "session_start", "user_id": "...", "days_since_signup": N }
{ "event": "churn_risk_flag", "user_id": "...", "days_inactive": N, "last_action": "..." }
{ "event": "reactivation", "user_id": "...", "trigger": "email|push" }
```

Signal: `d1_retention` = users with session on day 1 / signup_completed cohort
Signal: `d7_retention` = users with session in days 5-9 / signup_completed cohort
Signal: `d30_retention` = users with session in days 25-35 / signup_completed cohort
Signal: `monthly_churn` = users with 0 sessions in last 30d / active_last_60d
Signal: `wau_mau_ratio` = unique_users_7d / unique_users_30d

### 2.5 Revenue Signals

```
{ "event": "pricing_page_view", "user_id": "...", "plan_shown": "starter|growth|pro" }
{ "event": "upgrade_intent", "user_id": "...", "plan": "...", "trigger": "organic|paywall|email" }
{ "event": "subscription_upgraded", "user_id": "...", "plan": "...", "mrr_delta": N }
{ "event": "subscription_cancelled", "user_id": "...", "reason": "too_expensive|not_useful|other" }
```

Signal: `pricing_to_upgrade_rate` = `subscription_upgraded / pricing_page_view` (7d rolling)
Signal: `paywall_conversion_rate` = upgrades triggered by paywall / paywall hits
Signal: `cancellation_reason_distribution` = breakdown of `subscription_cancelled.reason`

### 2.6 Referral Signals

```
{ "event": "invite_sent", "user_id": "...", "channel": "email|link|sms" }
{ "event": "invite_converted", "referrer_id": "...", "invitee_id": "..." }
```

Signal: `viral_coefficient` = `invite_converted / signup_completed` (30d rolling)
Signal: `invite_conversion_rate` = `invite_converted / invite_sent` (30d rolling)

---

## 3. BIOISO Domain Configuration

Each domain below corresponds to one `BIOISOSpec` entry in the Invellum BIOISO runner.
The `metric_bounds` are the T1 Polycephalum rules. The `telos` is the T5 direction.

### 3.1 Domain: `onboarding`

**AARRR stage:** Activation
**Primary signal:** `activation_rate`
**Secondary signal:** `activation_time_p50`, `onboarding_drop_step`

```
metric_bounds:
  activation_rate:        min=0.25  target=0.35  ceiling=0.60
  activation_time_p50:    min=0.5h  target=12h   ceiling=48h  (lower is better)
  onboarding_completion:  min=0.50  target=0.70  ceiling=0.90

telos: maximize activation_rate while minimizing activation_time_p50
bounded_by: activation_rate ≤ 0.60 (ceiling; gains above this are noise)

mutation_space:
  - onboarding_step_order (permutation of: profile, goal, discover, connect, post)
  - cta_copy per step (Cialdini archetype: commitment, social_proof)
  - progress_indicator_style (steps|bar|percentage)
  - goal_selection_options (free_text|presets|both)
  - skip_step_allowed (bool per step)
```

**T1 rule:** If `onboarding_drop_step = "connect"` for 3 consecutive days, move `discover`
before `connect` in step order.

**T2 trigger:** If `activation_rate < 0.25` for 5 days, anneal step order permutations.

**T5 genome target:** Step order + CTA copy compound mutation across 2+ generations.

### 3.2 Domain: `feed_engagement`

**AARRR stage:** Retention (habit formation)
**Primary signal:** `loop_health`, `wau_mau_ratio`
**Secondary signal:** `feed_engagement_rate`, `session_depth`

```
metric_bounds:
  loop_health:            min=0.40  target=0.60  ceiling=0.85
  wau_mau_ratio:          min=0.20  target=0.35  ceiling=0.55
  feed_engagement_rate:   min=0.08  target=0.15  ceiling=0.30
  session_depth:          min=3     target=6     ceiling=12

telos: maximize loop_health sustained over 30-day cohort windows
bounded_by: feed_engagement_rate ≤ 0.30 (ceiling; above indicates gaming/spam)

mutation_space:
  - feed_ranking_weight_recency   (0.1 – 0.6)
  - feed_ranking_weight_network   (0.1 – 0.5)
  - feed_ranking_weight_relevance (0.1 – 0.5)  -- must sum to 1.0
  - new_connection_post_boost     (1.0 – 2.5x)
  - notification_digest_frequency (daily|3x_week|weekly)
  - notification_trigger_threshold (1|3|5 new items before push)
  - feed_item_count_per_load      (5|10|15|20)
  - show_suggested_connections_in_feed (bool)
```

**T1 rule:** If `session_depth < 3` for 7 days, reduce `feed_item_count_per_load` to 5 and
increase `new_connection_post_boost` by +0.25.

**T3 signal rewire:** If `notification_open_rate < 0.10`, wire `session_depth` signal to
`notification_digest_frequency` controller.

### 3.3 Domain: `retention`

**AARRR stage:** Retention (churn defense)
**Primary signal:** `d7_retention`, `d30_retention`, `monthly_churn`
**Secondary signal:** `reactivation_rate`

```
metric_bounds:
  d1_retention:   min=0.55  target=0.65  ceiling=0.85
  d7_retention:   min=0.25  target=0.35  ceiling=0.55
  d30_retention:  min=0.12  target=0.20  ceiling=0.40
  monthly_churn:  min=0.02  target=0.05  ceiling=0.08  (lower is better)

telos: minimize monthly_churn while maintaining d7_retention ≥ target
bounded_by: monthly_churn ≥ 0.02 (floor; near-zero churn is measurement error)

mutation_space:
  - churn_risk_threshold_days     (7|10|14)  -- days inactive before flag
  - reactivation_email_delay      (1|3|7 days after flag)
  - reactivation_email_copy_type  (feature_highlight|social_proof|question|discount_offer)
  - push_reactivation_enabled     (bool)
  - winback_offer_type            (none|free_week|feature_unlock|badge)
  - d7_nudge_enabled              (bool -- send "you have X unread connections" at day 6)
```

**T1 rule:** If `monthly_churn > 0.08`, activate `d7_nudge_enabled = true` immediately.

**Hard constraint (SafetyChecker equivalent):** Any mutation that reduces `d1_retention` below
0.55 is rejected unconditionally regardless of T-tier. This is the inviolable Skok floor.

### 3.4 Domain: `network_growth`

**AARRR stage:** Referral + Activation (network effects)
**Primary signal:** `viral_coefficient`, `invite_conversion_rate`
**Secondary signal:** `connection_rate` (connections made per DAU per week)

```
metric_bounds:
  viral_coefficient:       min=0.05  target=0.20  ceiling=0.80
  invite_conversion_rate:  min=0.10  target=0.25  ceiling=0.50
  connection_rate:         min=0.20  target=0.50  ceiling=1.50  (per DAU per week)

telos: maximize viral_coefficient sustained over 30-day windows
bounded_by: viral_coefficient ≤ 0.80 (ceiling; viral loops above this distort signal quality)

mutation_space:
  - invite_prompt_timing         (post_first_post|day_3|day_7|after_first_connection)
  - invite_copy_archetype        (reciprocity|social_proof|scarcity)
  - suggestion_algorithm_weights (industry_match|stage_match|mutual_connections)
  - suggestion_count_on_discover (3|5|8|12)
  - connection_request_copy      (formal|casual|shared_context)
  - @credibility_safe required on all copy mutations (§1.5)
```

**T1 rule:** If `viral_coefficient < 0.05` for 14 days, set `invite_prompt_timing =
post_first_post` and `invite_copy_archetype = reciprocity`.

### 3.5 Domain: `conversion`

**AARRR stage:** Revenue
**Primary signal:** `pricing_to_upgrade_rate`, `paywall_conversion_rate`
**Secondary signal:** `cancellation_reason_distribution`

```
metric_bounds:
  pricing_to_upgrade_rate:   min=0.03  target=0.08  ceiling=0.20
  paywall_conversion_rate:   min=0.05  target=0.12  ceiling=0.30
  upgrade_mrr_delta_avg:     min=0     target=29    ceiling=99   (USD per event)

telos: maximize paywall_conversion_rate without increasing cancellation_reason=too_expensive
bounded_by: upgrade_mrr_delta_avg ≤ 99 (ceiling; above this is enterprise, not B2C)

mutation_space:
  - pricing_page_social_proof_type  (logos|testimonials|counts|press)
  - pricing_plan_highlighted        (starter|growth|pro)
  - annual_discount_displayed       (bool, and discount_pct: 10|15|20|25)
  - paywall_trigger_page            (messaging|analytics|export|advanced_search)
  - paywall_copy_archetype          (authority|scarcity|reciprocity)
  - trial_offer_enabled             (bool)
  - trial_duration_days             (7|14|30)
```

**T1 rule:** If `cancellation_reason_distribution[too_expensive] > 0.40`, do NOT propose
mutations that increase price anchoring. Mutation space restricted to value-demonstration only.

---

## 4. Mutation Evaluation Protocol

BIOISO evaluates each proposed mutation against this protocol before committing:

1. **Isolation check:** Only one variable changed from previous generation. If a genome
   proposes multi-variable changes, decompose into atomic mutations.

2. **Gate hierarchy:** A mutation passes if it improves its primary signal without degrading
   any earlier AARRR gate. Gate order: Acquisition → Activation → Retention → Revenue → Referral.

3. **Cialdini isolation:** Only one Cialdini archetype per page per generation (§1.5).

4. **JTBD social safety:** Copy mutations require `@credibility_safe` annotation (§1.6).

5. **Skok floors:** `d1_retention ≥ 0.55`, `d7_retention ≥ 0.25`, `monthly_churn ≤ 0.08`.
   These are hard stops — no T-tier overrides them.

6. **Observation window:** Minimum 7 days of signal data before promotion. T5 meiosis
   proposals require 14 days across both parent generations.

7. **Lineage logging:** Every accepted mutation logged to `genomes/invellum/` with the
   signal delta that justified it. Rejected mutations logged with rejection reason.

---

## 5. Benchmarks (Industry Baselines)

These are the external reference points against which Invellum's evolution is measured.
Sources: Amplitude Product Benchmarks 2023, David Skok SaaS Metrics 2024, Andreessen Horowitz
Consumer benchmarks, Mixpanel's Product Benchmarks report 2022.

| Metric | Weak | Average | Strong | Target for Invellum |
|--------|------|---------|--------|---------------------|
| Activation rate | < 20% | 25–40% | > 50% | 35% |
| D1 retention | < 40% | 55–65% | > 75% | 65% |
| D7 retention | < 15% | 25–35% | > 45% | 35% |
| D30 retention | < 8% | 15–25% | > 35% | 20% |
| WAU/MAU ratio | < 0.20 | 0.25–0.40 | > 0.50 | 0.35 |
| Viral coefficient | < 0.05 | 0.15–0.30 | > 0.50 | 0.20 |
| Upgrade rate | < 2% | 5–10% | > 15% | 8% |
| Monthly churn | > 10% | 4–7% | < 3% | < 5% |

A BIOISO generation is considered successful if it moves at least one primary signal toward
"Strong" without moving any signal below "Weak". A generation is considered breakthrough if it
moves a primary signal past "Strong" without degrading any secondary signal.

---

## 6. T5 Meiosis Specification

MeiosisEngine genome recombination for Invellum follows the standard GS T5 protocol with these
Invellum-specific parameters:

- **Parent selection:** Top 2 domains by cumulative signal improvement over last 3 generations
- **Crossover point:** Between `mutation_space` parameter groups (not within a group)
- **Novelty guard:** Jaccard similarity ≤ 0.65 from any previous genome in the lineage
- **Compound verification:** Promoted genome must show signal improvement in 2 consecutive
  7-day windows (not just one spike)
- **Generation log:** `genomes/invellum/gen{N}/{domain}_{timestamp}.loom` — one file per domain
  per generation, in standard GS format with `-- GS EVOLUTION SPEC` blocks
- **Stagnation trigger:** If no domain improves its primary signal for 20 ticks, T5 fires
  a cross-domain `StructuralRewire` — wiring a signal from a succeeding domain into a stagnant one

---

*This document is the single source of truth for Invellum BIOISO fitness. Any change to signal
definitions, metric bounds, or mutation spaces must be reflected here before being applied to
a domain spec. The genome lineage is auditable against this document.*
