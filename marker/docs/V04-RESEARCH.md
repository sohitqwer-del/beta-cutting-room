# BETA Marker v0.4: what other systems do

> Historical research and proposed backlog, not today's feature matrix.
> Later releases add editor tools, browser exports and cutting-room work;
> see [features](FEATURES.md), [roadmap](ROADMAP.md) and [0.7](RELEASE-0.7.0.md).

Research date: **19 October 2026**. Branch: `marker/research4`.

## Decision

Build a production desktop release around the existing Rust core and TypeScript UI. Use **Tauri 2**. Put algorithm work into constrained compaction, repeated piece clusters, and local repair. Make the daily editor, marker reuse, printing, and reports dependable.

The commercial advantage is wider than automatic placement. Operators can rescue a layout, reuse an approved marker, run a queue overnight, and send an exact cut package to production. These are concrete v0.4 targets. A new learning model is a research experiment, not a release dependency.

### How to read the evidence

- **Vendor claim:** published by the supplier. It is not an independent benchmark.
- **Research result:** measured in the cited paper, on its own instances and hardware.
- **BETA evidence:** recorded in this checkout. Some older plans lag the implementation.
- **Target:** an engineering acceptance goal proposed here. It is not a predicted or measured saving.
- **Not verified:** the public material checked did not establish the fact. This does not mean the feature is absent.

Prices are public USD figures unless stated otherwise. They exclude any unlisted tax, hardware, support, and regional terms. A dated release and an undated current product page are identified separately. Search snippets and crawl dates are not treated as release dates. No current, independent, equal-rule comparison across the commercial engines was found.

## 1. Start from BETA's actual position

Read: [PLAN.md](PLAN.md), [CUTTING-ROOM-PLAN.md](CUTTING-ROOM-PLAN.md), [benchmarks.md](benchmarks.md), and [V02_RESULTS.md](../crates/beta-marker-nest/V02_RESULTS.md). Also checked the [current UI report](../apps/web/UI3_REPORT.md), [UI README](../apps/web/README.md), [matching results](../crates/beta-marker-match/README.md), CLI batch code, and editor code.

| Evidence | What it permits us to conclude |
|---|---|
| Lectra stratified sample: 300/300 valid at 60 s; mean **-2.3556 efficiency points** | BETA has a useful baseline. Large markers need work. This balanced sample is not a population estimate. |
| Matched-duration subset: 60/60 valid; mean **-1.0865 points**; budgets 244–608 s, median 304 s | More time helps. The machines and background load are not matched. “Equal time” is conditional, not hardware-normalized. |
| On that same subset, 60 s to matched duration adds **1.4411 points** on average | Longer production budgets merit a product feature and controlled measurement. |
| 151+ pieces, reference efficiency 75–85%: **-6.8475 points** at 60 s | Optimize by instance size and constraint class. A single overall mean hides the worst production cases. |
| Simon, 60 s: 1/3/5 concurrent runs score **89.02/88.77/88.24%** | Random seed portfolios already lost this local test. Fewer separator workers and extra coordinators confound the comparison. |
| Teagan: open **78.56%**, tubular **69.83%**, folded **40.90%** | The folded path needs a different search. These layouts have different freedoms, so 78.56% is not a legitimate folded target. |
| Simon shade fixtures lose **7.08** points with length bands and **10.59** with width lanes | Some loss is required by shade rules. Current partitioning can also leave avoidable empty space. Measure those parts separately. |
| Gemini comparison in the task brief: BETA 4.6–6.7% shorter on one style | User-supplied context. It is not evidence that BETA beats the market, or an independent proof that Gemini is weak. |

The nesting report says the obstacle/fold route uses construction search rather than Sparrow compression. The shade implementation divides whole bundles into sub-markers. Those architectural differences are more actionable than another seed sweep.

The current app already includes Pattern, Digitize, Editor, Cut plan, Rolls, Bundles, Compare, and Help. Roll inspection, shade/shrinkage groups, allocation, tickets, re-cuts, costing, and actuals reports exist. Multi-width batch trials exist in the CLI. Do not spend v0.4 rebuilding them from the older plan. Turn them into a durable operator workflow.

The editor already moves, rotates, locks, validates, and packs one piece left. Free rotation advances by 15 degrees. Pack-left follows a checked path, with a 1.5 s cap. This is not yet a full bump, group compact, swap, fine-tilt, or marker-library toolset. The UI report records uncalibrated costing, generic ISO output, no hardware print calibration, and browser-based draft storage. Those are adoption gaps.

## 2. Commercial marker systems in 2025–2026

### 2.1 Nesting, deployment, and claims

“AI” below describes the supplier's wording. It does not establish neural placement, reinforcement learning, GPU use, or a public algorithm.

| Product and current public evidence | Nesting approach and time controls | Deployment and claimed gain | Operator functions worth copying |
|---|---|---|---|
| **Tukatech TUKAmark + SMARTmark / Net-Q**. Current [SMARTmark page][T1]; dated [25 March 2025 guidance][T3]. | Proprietary automatic nesting; blocks, matching/grouping, half-run option. Net-Q shares one queue across stations. No public learning architecture or multicore scaling result verified. Guidance: one minute per six production pieces; costing 15 pieces/minute. | Windows workstation/server queue. Internet-connected stations do not prove that nesting is a cloud service. Testimonials claim 1% over other software and 3% over a customer's best core-style markers. Uncontrolled vendor claims. | Partial work, grouping, stripe/plaid matching, yield/costing reports, cut-data preparation. Legacy tutorial documents size sections and bump lines. [Section tutorial][T4]. |
| **TUKAcad 2026 / TUKAcad-GC**. The current site names 2026; [May 2026 announcement][T5] describes GC, StyleHub, and MES connections. | Pattern, grade, marker preparation, automatic pattern generation via APM. No public 2026 nesting release note or measured “AI nesting” increment verified. Do not confuse automatic pattern generation with marker optimization. | Installed CAD with subscription/service portal and connected collaboration. The [current package page][T2] publishes prices. | Integrated fit/pattern-to-marker data, measurement charts, digitizing, help in several languages, production exports. |
| **Gerber AccuMark / AccuNest**. Public [2025.1/2025.2 history][G1] and [2026.1 release PDF][G2]. | AccuNest documents Draft and time-per-marker/per-piece Full modes. PE has one nest/pack engine; Expert has two nesting engines plus a pack engine. Exact 2026 threading and learned-model details are not disclosed in the checked material. [AccuNest guide][G3]. | Windows CAD and nesting server; optional Lectra cloud submission. No fair 2025–26 efficiency increment verified. The public product page identifies 2026.1 as current. [Product][G4]. | Sliding, compact marker, split piece, lay rules and reusable placement relationships. 2025 adds batch cut tickets and cloud sections by model. 2026.1 fixes buffer/construct saving and matching, adds active-grid cloud matching and clearer sliding. |
| **Gerber AccuPlan**. [Workflow][G5] and [marker-library tutorial][G6]. These are established capabilities, not a dated 2026 release. | Cut-order quantities, utilization estimates, marker selection, spreading/cutting choices; calls AccuNest. It is a planning layer, not a different packing algorithm. | Desktop integrated with AccuMark/AccuNest. Current price and a separate 2025–26 solver gain not verified. Older tutorial OS/SQL requirements must not be used as current installation advice. | Marker library, reuse from storage, cut tickets, work orders, alternative plan results and statistics. |
| **Lectra Diamino / Quick Nest / Flex Offer**. Historical [Flex Offer specification][L1]; [2024 customer case][L2]; newer platform below. | Proprietary automatic nesting. Flex has Urgent, Standard and Strategic tracks, used together. The customer case describes strategic 1/4/12-hour options. Quick/Flex cloud submission is integrated into AccuMark. [History][G1]. | Diamino Expert is the local lineage; Quick/Flex provide cloud workflows. Do not call them discontinued without supplier confirmation. Case reports faster costing, not a controlled solver win. | Priority lists, high-volume unattended jobs, format conversion, sectional and shade-zone nesting, standard laying recipes. |
| **Lectra Valia Fashion**. [June 2025 Estimate article][L3]; [April 2026 features][L4]; [May 2026 waste article][L5]. | Explicit AI claims for fabric estimation and, in 2026, nesting. Public pages describe scenarios, constraints, motif/panel management, distortion correction and equipment handoff. They do not disclose an algorithm or GPU architecture. | Cloud platform linking orders, planning, nesting and cutting. “Up to 30% waste reduction” is a motif-heavy production-line claim, not 30 efficiency points or a general solver result. | ERP order intake, production recipes, estimation scenarios, any-vendor equipment connections, actual-versus-plan KPIs. Include Valia in any new competitive brief; Diamino alone is out of date. |
| **Optitex Marker / Nest++ Pro / Match++**. [O/25 announcement][O1], [O/26 release][O2], [Marker page][O3], [module brochure][O4]. | Multicore nesting within an engine and a separate parallel-marker module. O/25 and O/26 claim improved algorithms. Some suite marketing calls nesting AI-assisted; no model disclosure or fair numerical gain. Match++ handles stripe/plaid/flow matching. | Installed Marker; connected cloud products are separate. No evidence that all Nest++ Pro calculations run in the cloud. No public time/quality curve found. | Rotate/flip, including fold work; damaged zones; reports. O/26 adds batch vertical bump lines, grouping/shading/overlap/mixing/bundle controls and automated PDF/DXF output. Separate modules include spacing optimization and compaction. |
| **Audaces Marker / Supera / Supera Max**. [Supera][A1], [FAQ][A2], [lifecycle notice][A3]. | AI-branded server; multiple algorithms, queue priorities, background processing for several stations. No learning design, core count or time-budget curve verified. | Desktop preparation plus dedicated processing server. Cloud storage is not proof of cloud nesting. Vendor claims up to 3% better digital markers with the coprocessor. Old coprocessor ended sales in 2022/support in 2023; replacement is Supera Max. | Grain/rotation/print restrictions, DXF intake, queue prioritization and freeing the operator's PC. Actual-consumption monitoring is in the broader ICF suite. |
| **Style3D \| ASSYST Automarker**. [Current Automarker description][S1]. | Parallel cloud processing; customizable laying rules; size-combination comparison. No public algorithm, neural-model details, fixed time tiers, or independently measured efficiency advantage found. | Cloud service connected to Assyst.CAD or uploaded data. Free four-week trial advertised; ongoing price not published on the checked page. | Dynamic fusing blocks, horizontal quality zones, vertical bundle zones, saved laying rules, production-partner delivery, plot/cut/PDF package. |
| **Style3D Studio Super Marker**. [October 2025 V9.1 help][S2], [current Marker manual][S5] and [2026 V10 announcement][S3]. | Maximum nesting time 0–60 minutes, default 0.5; mixed-size, single-size and grouped markers with stagger controls. AI automatic arrangement and auto-sewing also concern 3D garment preparation; they are not proof of a better strip-packing optimizer. | Studio desktop with connected services; purchased Super Marker dongle enables the separate engine. No matched nesting efficiency claim verified. | QR code plus readable text, per-piece consumption, grouped size handling, batch output and nesting preferences. |
| **CLO Print Layout / Nest Patterns**. [Current manual][C1], including 2026.1 notes. | Stop-at-any-time nesting; re-nest one fabric; size quantities, 1/2/4-way grain, one-bundle-one-direction, directional buffers, fixed placements and texture/print matching. No public engine/core/GPU disclosure or fair commercial nesting result. | Desktop CLO on Windows/macOS; CLO-SET is a related collaboration product. Do not infer cloud nesting. | Collision-aware manual move, arrow-key nudge, adjacent selection, per-fabric layout, labels and consumption display. Manual has both older and 2026.1 outline controls; test the installed version before writing interoperability rules. |
| **Richpeace GMS / New Super Nest V10**. [Super Nest][R1], [CAD V10][R2], [manual][R3]. Product pages are not dated 2025–26 release notes. | New 64-bit algorithm; mixed, garment, size, same-piece and custom groups; distance sections, defect avoidance and special angles. “64-bit” is not evidence of multicore or AI. | Windows installed software. Claims faster/high-utilization layouts; no reproducible numerical gain verified. | Manual/interactive rescue, paired/reversed duplication, group/shade sections, defect avoidance. Older manual documents time/efficiency stopping and hole filling. |
| **BOKE/BOK V22 AI Super Nesting**. [Public branded catalogue][B1]; no dated release note verified. | Proprietary AI-branded version; separate four-core Super Nesting 7.0 offer. No disclosed learning system or public benchmark. | Catalogue presents installed software. Claims 1–3% higher utilization than similar/ordinary engines, with no comparable data. | Marker preparation, print/input, DXF conversion, PLT opening, cutting files, cut-plan interface and print alignment. Detailed bump/swap controls not verified. |
| **ET / ETSYSTEM**. [Official site][E1], [2021 integration description][E2], [2023 technical review][E3]. No detailed official 2025–26 nesting changelog found. | Automatic nesting plus cut-plan/spread integration and data APIs. Public older material does not establish a current engine supplier, GPU, multicore count, or neural method. | Installed CAD plus online/data-service architecture. Current precise deployment/licensing terms need a supplier quote. | Order intake, automatic lay creation, checks, serial-number lists and production-data exchange. Current fine-edit tool inventory not verified. |
| **Gemini Nest Expert / server / cut planning**. [Current cutting-room page][M1], [older Nest Expert sheet][M2]. | Proprietary multicore optimization and queued processing. The older sheet names 2/4/8/16 cores and an autonomous server version. No current learned-model or GPU claim verified. | Windows CAD/server lineage; current workflow deployment terms need confirmation. Older vendor post claims 2–10% versus manual/older CAD and 27% faster than its prior version. That is not a 2026 or cross-vendor benchmark. [Post][M4]. | Groups, fusing, shade, defects, visual plaid/check matching, planning and equipment/enterprise exchange. Use its real saved production markers as comparisons. |

### 2.2 Files and price: what is actually public

Format support is not lossless semantic support. Grain, handedness, grade rules, notches, matching links and units need an import/export acceptance corpus. A native converter may be an extra module.

| Product | Public format evidence and limits | Public price checked |
|---|---|---|
| TUKAcad / TUKAmark / SMARTmark | Current package lists import/export, digitizing and cut data; SMARTmark explicitly lists PDF plot. Current complete AAMA/ASTM/native converter matrix not verified. [T1][T1], [T2][T2]. | TUKAcad Professional **$199/month**, including TUKAmark; $1,025/6 months. Pattern-only Professional $99/month. Learning $19/month has limits. **SMARTmark/Net-Q extra: quote**. Do not mistake the separate $199 training course for a perpetual CAD licence. |
| AccuMark / AccuNest / AccuPlan | Native AccuMark storage; documented Modaris MDL conversion, marker plot PDF, and cut tickets. Full DXF/converter and cutter profiles require installed-version checks. [Release history][G1], [brochure][G7]. | Quote. Product page describes subscription commitments of 1–3 years. No public current AccuNest/AccuPlan per-seat figure verified. |
| Lectra Diamino / Quick / Flex / Valia | Flex Pattern Converter accepts other CAD data; Valia describes any-vendor equipment connections. Exact native versions and licensed format modules require a quote/sample test. [Flex][L1], [Valia][L6]. | Quote / usage or subscription contract. No reliable public 2025–26 unit tariff verified. |
| Optitex | PDS/Marker native data; O/26 confirms PDF/DXF batch output. Match++ is a separately identified module. Full HPGL/ISO/cutter options depend on suite/configuration. [O/26][O2], [modules][O4]. | Quote. No public current Nest++ Pro/Match++ seat price verified. |
| Audaces | Official FAQ confirms DXF pattern intake; native ecosystem and cutting/plotting workflow. Current complete export/profile matrix not published in the pages checked. [FAQ][A2]. | Quote. Audaces360 business tiers bundle Pattern/Marker/Supera differently. [Plans][A4]. |
| ASSYST Automarker / Style3D | Automarker assembles marker, plot, cut and PDF packages; generic CAD exchange claimed. Older terms name DXF/HPGL/ISO. Studio documents BOM output, vector PDF, TIF/PNG/JPG; its Marker overview also names PLT/HPGL production. Confirm the engine/version's actual export list. [Automarker][S1], [terms][S4], [Studio][S2], [manual][S5]. | Quote; Automarker advertises a four-week trial. Studio/Automarker are not one interchangeable licence. |
| CLO | Native project plus DXF, PDF and PLT export in the product/API. Do not assume DXF preserves the Print Layout nesting arrangement. [API][C2]. | Individual **$50/month or $450/year**; student $25/$225. Enterprise **quote**. These are not a factory multi-seat quotation. [Pricing help][C3]. |
| Richpeace | Current CAD page names DXF, AAMA/ASTM/AutoCAD; GMS reads its design/grading files. Plot/cut output supported; test exact equipment dialect. [CAD][R2], [manual][R3]. | Quote. No current official list price verified. |
| BOKE | Catalogue explicitly names DXF conversion, PLT opening and cutting-file output; exact dialects unspecified. [Catalogue][B1]. | Advertised **$7,500/set** for V22 AI; **$9,200/set** for four-core 7.0. Undated catalogue, not a verified India quotation. Support, tax and licence duration not clear. |
| ET | Native ET files, online conversion and standard data/API exchange. Current exact DXF/cut dialect matrix not verified from official pages checked. [ET][E1], [interfaces][E2]. | Quote. No verified current official public amount. |
| Gemini | Older Nest Expert sheet names DXF/DXF-AAMA and native CAD intake, HPGL/ISO-CUT/RS274d output, OTX ERP exchange, and PDF nesting via AutomART 2Nest. Current site also names PDF/AI print imposition. Version coverage needs samples. [Expert][M2], [current workflow][M1]. | Quote. No current public factory licence amount verified. |

### 2.3 New entrants and adjacent tools

| Entrant | Verified public proposition | What remains unproven |
|---|---|---|
| **Gelian / Seamline Ultranest / Marka**, Sri Lanka | Browser-native, AI-branded marker preparation from text, version history, roll-width suggestions, cut planning and consumption-to-PLM/BOM. [Supplier][N1]. | 85–90% is an advertised target; the site labels financial examples illustrative. No independent dataset/time/rule comparison, algorithm, file matrix or public price verified. Request a real same-job trial before treating this as an efficiency leader. |
| **FashionInsta** | AI trained on a customer's DXF archive; site shows “Pattern Nesting” among its outputs. [Supplier][N2]. | A production marker editor, rule coverage, delivered packing service, time budget, benchmark and price were not established. This is a lead to investigate, not a verified marker competitor. |
| **Six Atomic Synthesis** | Automated/modular pattern development and AI grading; material width and shrinkage settings. [Supplier][N3]. | No evidence from this page that it replaces a factory marker editor or beats specialist nesting engines. |
| **Sparrow Studio / Tetrinest / Deepnest** | Sparrow has a local browser interface. Its paper names Tetrinest as a direct-to-film benchmark contributor. Deepnest supports part-in-part placement and DXF/SVG. [Sparrow repository][P2], [paper][P1], [Deepnest][N4]. | These are not all new AI garment startups. UV/DTF/laser nesting does not establish garment grain, nap, shade, paired-demand or factory planning coverage. |

### 2.4 What to take from the market

The useful pattern is **prepare → queue → inspect/edit → approve → plot/cut → reconcile**. Net-Q, Supera, Automarker, Gemini's server and Flex all make unattended throughput part of the product. AccuPlan and Valia sell plan quality and data continuity as well as marker density.

The most concrete recent editor evidence is Optitex O/26's batch bump/group/shade controls and AccuMark 2026.1's fixes to real matching/buffer cases. These suggest spending on repeatability and operator control. “AI” on its own does not explain the competitive gap.

TUKAcad 2026 GC deserves a separate watch item. Official demonstrations describe applying base-size stitching/3D placement across graded sizes and a `.TUX` file carrying stitching, size, placement, piece and grading information. These are garment-development features, not demonstrated marker-density gains. [Official TUX post][T6], [founder demonstration][T7].

Hardware claims also need context. SMARTmark's current recommendation includes Windows 11 Pro, a 16-core Ryzen 9 and 32 GB RAM. It is a recommended configuration, not a verified minimum. BETA's opportunity is a predictable queue and responsive editor on modest factory PCs; publish the measured quality/time tradeoff on those PCs. [Hardware table][T1].

## 3. Nesting research: what can change the result

An irregular strip packer places non-rectangular pieces in a fixed-width strip and minimizes its length. Good collision detection makes search cheaper. It does not choose a good arrangement by itself. A **percentage point (pp)** is an absolute change in efficiency. A relative reduction in marker length is a different measure.

### 3.1 Relevant work, 2024–2026, plus necessary older foundations

Effort below is an estimate for one experienced engineer familiar with BETA. It includes implementation and a useful benchmark, not full product hardening. Gains are evidence-specific. **There is no defensible universal expected gain for these techniques on BETA.** Proposed numeric gates appear separately in section 6.

| Technique / source | What the evidence establishes | Expected BETA benefit and effort | Available code / licence |
|---|---|---|---|
| **Sparrow**, 2025 paper, revised v4 on 6 October 2026. [Paper][P1], [implementation][P2]. | Fixed-length feasibility search resolves overlaps; exploration and compression. Academic experiments use 100 runs of 20 minutes, not 60 s. Strong results on garment instances; table below. | Already BETA's main solver. First find adapter/search-budget losses. Effort **1–2 weeks** for controlled phase, worker and raw-engine comparisons. No automatic gain from an upgrade alone. | Rust, **MIT**. BETA already pins 0.3.0. |
| **Sparrow 0.2/0.3 follow-up releases**, September/October 2026. [0.2 release][P3], [0.3 release][P4]. | 0.2 reports median **33.6% more evaluations/s** on 17 paired Apple Silicon SIMD cases; range 9.7–54.0%. 0.3 changes integration/input APIs. Throughput is not efficiency. | BETA's 0.3 already follows that release. Do not claim a new 33.6% speedup. Profile x86 Windows before choosing SIMD or a lower-level change. **1–2 weeks**. | Same MIT project. No separate, verified 2026 garment solver that supersedes Sparrow was found. |
| **Jagua collision engine**, 2025 journal/manuscript, current 1.1 release. [Paper][J1], [code][J2], [1.1 notes][J3]. | Spatial indexing and cheap circle/line surrogates reject collisions before exact geometry. 1.1 exposes generic hazard entities and quality-zone overlap targets. | Strong fit for constrained search. Register defects, fixed pieces and quality zones rather than abandoning the fast collision path. No measured BETA density gain. **2–4 weeks** for a prototype; API migration needs care. | Rust, **MPL-2.0**. Modified covered files retain MPL obligations. BETA's separate files can remain Apache-2.0. 1.1 is not a drop-in Sparrow dependency upgrade. |
| **Repeated pair/cluster placement**, Elkeran 2013; [publisher DOI][P5]; [2018 clustering study][P6]. | Pairwise clustering reduces placement decisions. The 2018 study reports benefit only on some cases. Sparrow also identifies unexploited repeated structure. | Test sleeve pairs, trouser legs and recurring small-part groups. Cache several compact arrangements, place them, then allow breakup/refinement. Best chance on repeated styles; can hurt mixed layouts. **3–4 weeks**. | No licensed implementation of Elkeran's full method verified. Implement the idea independently. Article access is not a code licence. |
| **Hole filling and local gap removal**, [GFPack++ enhancement study][F1]; [Richpeace manual][R3]. | GFPack++ garment enhancement changes **74.25% to 77.04%** for its own generated layouts. This is not a gain on Sparrow. Richpeace documents filling small pieces into spaces between placed pieces. | Search empty pockets and the marker tail. Remove a few small pieces and reinsert them around a fixed neighbourhood, then compact. **2–4 weeks**. Distinguish spaces between pieces from internal polygon holes. | BETA can implement independent Rust repair using existing geometry. GFPack++'s root has no verified code licence; do not copy its implementation. |
| **Overlap-minimization variants**: raster penetration maps, multiresolution and coordinate descent. [2015 GPU/multiresolution paper][P7], [2022 coordinate descent][P8]. Older foundations remain relevant. | Grid-based overlap loss can use matrix operations/GPU parallelism. Multi-resolution changes the speed/precision tradeoff. Coordinate descent offers a different local move policy. | Prototype on difficult constrained neighbourhoods, not a wholesale raster replacement. Grain/zone penalties need separate terms. Any approximate result needs exact validation. **4–6 weeks**; benefit unknown. | No reusable licensed full implementation verified from these sources. Independent implementation required. |
| **GPU collision acceleration**, recent [ESICUP 2025 GPU abstract][P9] plus CPU evidence above. | The conference abstract concerns **3D voxel strip packing**, with preliminary timing evidence. It is not a 2D garment result. | Low v0.4 priority. Irregular branches, geometry transfers and small batches may erase GPU gains. Benchmark collision-batch throughput first. **6–10 weeks**, driver/deployment work extra. No honest garment pp forecast. | No licensed code accompanying that abstract verified. CUDA would also impose hardware/runtime constraints. |
| **Learning layout/placement**, GFPack++: 2024 preprint, ICCV 2025, October 2026 updated manuscript. [Paper][F1], [conference][F2], [code][F3]. | Attention-based gradient fields plus feasibility enhancement. On 48-piece garment tests: **77.04% / 10.5 s** at batch 512 versus SVGnest **72.75% / 80.21 s**. Training/testing server has four RTX 4090s; training converges at 50 h. Different data and rotations from BETA. | Interesting initializer or local-region proposal generator. No evidence it beats Sparrow, Lectra, or constrained garments. **8–12+ weeks**, including data/training and validator integration. Keep off the critical path. | Python/C++ and weights are published. **No root LICENSE or README grant verified** in the inspected repository. Obtain explicit code/weight/data terms before reuse. |
| **Learning rotations**, Abdou et al., online May 2025 / journal 2026. [Paper][F4]. | DRL rotation policy plus rule-based sequencing/placement. Authors report **97% less compute time** and **11% better utilization** against an open-source baseline on sheet-metal data. Not 11 pp; not a garment/Sparrow result. | A policy must obey BETA's very small allowed angle set, reducing this method's relevance. **8–12+ weeks**. No transferable saving established. | No public reusable code/licence established in the checked article. |
| **Learning-guided ordering / surrogate selection**, Jiang et al., online May 2025 / volume 2026. [Publisher][F5]. | Incremental SVR estimates candidate fitness within a multi-objective evolutionary method. Public abstract concerns four real laser-welded-blank instances and weld/material objectives. | Candidate ranking could reduce expensive nests, but evidence does not establish a garment-strip improvement. Start with a simple supervised ranking experiment only after collecting local traces. **6–10 weeks**. | Subscription article; reusable licensed code not verified. |
| **Multiple populations with migration**, [2017 μ-BRKGA][I1], and [April 2026 modular island method][I2]. | Older paper compares parallel population search with a different RKGA/placement scheme; it is not a clean equal-CPU island ablation. 2026 paper reports gains over its metaheuristic baselines; no Sparrow comparison or verified reusable code. | Diverse search policies and exchanging good sublayouts are more promising than identical seed races. **3–5 weeks** after local repair exists. Require equal hardware/time/total-thread tests. No guaranteed gain. | Full reusable code licence not verified for either. Article CC terms do not establish a code licence. |
| **Independent constructive engine**, PackingSolver. [Code][Q1], [irregular API][Q2]. | C++ library/CLI supports irregular shapes, bins, holes, defects, rotation/mirroring and separate spacing constraints. Does not establish native garment-rule parity. | Useful alternative initializer for defect/zone jobs and region repair. Adapt, validate, then warm-start BETA. **3–5 weeks**. Do not assume it beats Sparrow on ordinary strip jobs. | **MIT**. Audit transitive libraries/build settings at the pinned revision before shipping. |
| **Sparrow/Jagua ecosystem forks**, [ironnest][Q3]. | Repository claims f64 geometry, reproducibility, constructive placement, sliding compaction and guided separation. Its examples are not independent garment benchmarks. | Read for experiments; no reason to replace the validated core on those claims. **1 week** evaluation if needed. | Rust, **MPL-2.0 across repository**, not Apache/MIT. Treat it as a separate covered dependency or independently implement ideas. |
| **Small exact/local solver**, 2025 parallel branch-and-bound-and-prune preprint. [Paper][Q4]. | Authors solve 17 previously open **discrete-grid** instances to optimality. Discrete optimality is not continuous optimality. | Potential reference for tiny holes/regions and lower bounds. Not the main 150+ piece production solver. **4–8 weeks**. | Usable code licence not verified from paper record. |

### 3.2 Garment results that are worth keeping separate

| Sparrow paper: 20-minute experiments | Mean efficiency | Best reported | Prior best reported |
|---|---:|---:|---:|
| SHIRTS | 89.66% | 90.92% | 88.96% |
| SWIM | 78.26% | 79.83% | 75.94% |
| TROUSERS | 91.73% | 92.62% | 91.06% |

Source: [Sparrow, tables 2–3][P1]. Means come from 100 runs; “best” is not the expected result. Historical algorithms used their published conditions. These are academic garments, without BETA's full factory constraints. BETA's 60 s figures must not be presented as a reproduction of the 20-minute table.

Sparrow's ten new cases include four derived from Lectra fashion data. The paper calls repeated compact structures a blind spot. That is a research direction, not evidence that any specific BETA cluster will save a set number of points. [Paper, section 12][P1].

### 3.3 What actually beats a single run?

The strongest local evidence still favors **one adequately threaded run**. Neither the older population paper nor the 2026 island paper establishes a clean, equal-total-CPU win over BETA's current solver. Their configurations, move rules and baselines differ. A claimed theoretical linear speedup is not proof of that practical result.

There is practitioner evidence for **structural hybrid search**: the [fourth-place Santa 2025 solution write-up, February 2026][I3] combines Sparrow, manual structure and region replacement. The problem packs repeated tree shapes in squares. It supports investigating structured regions; it does not quantify garment savings or prove pure islands beat a single run. Competition code terms must be checked separately.

Run a bounded experiment with four arms: current single run; independent seeds; different move policies; different move policies with occasional sublayout exchange. Give each the same total thread budget and wall deadline. Include coordinators and both phase pools. Keep a shared valid incumbent. Publish every result, including regressions. Drop the portfolio unless it wins a held-out factory set.

### 3.4 A concrete constrained optimizer experiment

Use the same effective cut outlines as the independent validator. Keep the current valid incumbent while searching.

```text
valid marker
    -> choose tail, empty pocket, or crowded shade boundary
    -> remove 5–20 eligible pieces or a few whole bundles
    -> create several orders / pair clusters
    -> insert with exact collision checks
    -> separate and compact within fixed/quality/fold constraints
    -> validate complete demand and every garment rule
    -> retain only a shorter valid marker
```

For folded markers, fold pieces need exact edge contact and motion along that edge. Free pieces need 2D moves. Do not freeze the whole layout because some pieces have one-dimensional freedom. Test fold order, admissible full/half choices and repair in the spaces around edge pieces. Preserve the physical-layer quantity contract.

For shade, first keep the existing band boundaries fixed. Search whole-bundle assignments across them and re-nest affected bands. Mix small and large bundles. Optimize the longest lane and the final used length. Move boundaries only if the fabric's measured shade specification permits it. The unconstrained marker is a diagnostic lower bound, not a valid replacement.

Internal holes and spaces between pieces are different. Sparrow 0.3 rejects item holes. Do not silently close or discard holes. Exact support needs importer, geometry, validator, metrics and cutter semantics, with a compatible engine. Inter-piece pocket filling can ship without that extension. [0.3 migration][P4].

## 4. Ship a proper Rust desktop application

**Recommendation: Tauri 2 for v0.4.** Keep the existing TypeScript/SVG editor and Rust crates. Compile the heavy engine natively. A desktop shell does not make nesting faster by itself; removing unnecessary serialization and browser-worker limits may help only where profiling proves them.

### 4.1 Tauri versus native Rust UI

| Choice | Licence verified | Fit for this project | Main cost / limitation |
|---|---|---|---|
| **Tauri 2** | **MIT OR Apache-2.0**. [Project][D1]. | Reuses existing web UI. Native Rust backend, menus/dialogs, bundled assets, MSI/NSIS installers and file associations. Best route to a factory pilot. | WebView2 runtime and IPC need packaging/testing. Web rendering is not a Win32 widget toolkit. Printing needs a tested production path. |
| **egui / eframe** | **MIT OR Apache-2.0**. [Manifest][D2]. | Strong custom drawing/tool UI. Rust throughout; no WebView2. | Rebuild forms, SVG/editor interactions and accessibility behavior. Immediate-mode UI requires care with large tables, focus and text entry. No automatic factory-print workflow. Estimate **8–14 engineer-weeks** for comparable UI scope, with substantial uncertainty. |
| **iced** | **MIT**. [Repository][D3]. | Structured Rust application model and custom rendering. No browser runtime. | Full UI rewrite and new rendering/input/printing integration. Estimate **10–16 engineer-weeks** for comparable scope. No demonstrated BETA nesting benefit. |
| **Slint** | **GPLv3**, **Slint Royalty-free 2.0**, or paid commercial terms. [Licence overview][D4], [exact royalty-free text][D5]. | Declarative native UI; a credible future option for dedicated devices. A normal Windows PC desktop app can use the royalty-free route. | Rebuild the UI. GPL distribution would impose GPL terms on the combined distributed work. Royalty-free is a distinct licence, not Apache/MIT: attribution and restrictions apply. Estimate **10–16 engineer-weeks**. |

The Slint royalty-free licence permits integrating Slint in a desktop/mobile/web application with either the AboutSlint widget or an attribution badge on a public webpage. It excludes embedded systems and exposing Slint APIs, and does not permit distributing Slint alone under that licence. Keep its notices. Thus “Slint always needs a paid licence” is false; “Slint is permissively open source like MIT” is also false. Tauri is simpler for BETA's Apache-2.0 application and redistribution goals. [Exact terms][D5].

### 4.2 Windows packaging plan

| Area | Concrete v0.4 decision |
|---|---|
| Target PCs | Test **Windows 10 22H2 and Windows 11 x64** on 8 GB, integrated-graphics factory PCs as well as the development laptop. Use a conservative CPU baseline; do not ship an unconditional `target-cpu=native` build from the developer's PC. |
| Installers | Signed **NSIS setup.exe** first; MSI for managed IT installs. Stable application/upgrade IDs, Start menu entry, uninstall, and upgrade preserving user data. Tauri supports both formats. [Windows packaging][D6]. |
| Fully offline installation | Bundle WebView2's **offline installer**, not just its bootstrapper. Tauri documents about 127 MB extra, versus about 180 MB for a fixed runtime; actual release size varies. Check the runtime is present before launch. [Packaging][D6]. |
| WebView2 reality | Windows 11 includes it. Most Windows 10 PCs have it, but some do not. Microsoft explicitly says to handle missing installations. The Edge browser executable is not a substitute for the runtime. [Microsoft distribution][D7]. |
| Windows 10 support | Ordinary Windows 10 support ended in October 2025; ESU/LTSC status varies. Microsoft says Edge/WebView2 on 22H2 receive updates until at least October 2028, independently of ESU enrollment. Browser-runtime support does not restore OS support. [Microsoft lifecycle][D8], [WebView support][D9]. |
| Runtime assets | Bundle HTML, JS, fonts, WASM and help locally. No CDN, account or connection required for normal work. Browser mode may remain for development; desktop production jobs call native Rust commands. |
| Heavy work | Run nesting on bounded background workers or a managed engine process. Report progress by events. Support cancel, deadline and best-valid checkpoint. Never block the UI event loop with a nest, import or digitizer scan. |
| Files | Register a BETA-specific job/project extension, e.g. `.betamarker`; make DXF/PLT optional “Open with” choices. Do not hijack every `.json` or `.dxf`. Handle second-instance open requests, drag/drop, Unicode paths and removable media. [Association configuration][D10]. |
| Persistence | Store projects, library, queue and actuals in local files/SQLite with migrations. Atomic saves and recovery copies. Browser localStorage is not the sole production record. Keep exportable backups. |
| IPC | Pass handles/job IDs and progress deltas where practical; avoid resending all polygon vertices each frame. Scope filesystem/process permissions to required commands. Geometry validation remains in Rust. |
| Updating | Signed installer upgrades and an optional signed updater. Provide a USB/offline update path. Pin a tested runtime baseline; a fixed WebView2 build makes BETA responsible for distributing its security updates. |

### 4.3 Printing and cutting are separate deliverables

Tauri has a WebviewWindow print method. That is useful for reports; it is not evidence of reliable silent 1:1 plotter output, selected-printer control, or cutter compatibility. [Print API][D11].

Keep Rust-generated vector PDF/PLT/ISO as the production source. Add named printer/cutter profiles: media width, roll direction, units, plot scale, feed/margins, pen settings, label stock, and device dialect. Use the Windows print path for A4 reports and thermal tickets. Send HPGL or machine files through a verified spool/file workflow. Add explicit retry and completion states.

Pass a hardware acceptance test before marking a profile production-ready: a 1,000 mm reference line, known notch/drill geometry, a long marker with section boundaries, and a QR ticket scanned back to the same lay/bundle. Set the allowed dimensional error with the factory and machine specification. No generic ISO export can prove that every cutter will interpret it correctly.

## 5. Daily operator workflow: gaps to close

These are documented commercial workflow capabilities and proposed BETA improvements. This research did not observe factory shifts or establish how often each tool is used. Confirm shortcut conventions and report layouts with pilot operators. Do not assume that adding a checkbox makes a factory rule usable.

### 5.1 Marker editor

| Capability | Commercial evidence / operator purpose | BETA now and a concrete v0.4 increment |
|---|---|---|
| **Bump / slide to contact** | Gerber documents sliding against pieces and vertical/horizontal/diagonal bump lines. [Help][W1]. | One-piece pack-left exists. Add four-direction and drag-direction contact; collision-free path; gaps/buffers respected. Show the stopping contact. Support a selected group and one undo action. |
| **Compact selected region / whole marker** | Gerber has Compact Marker; Optitex lists Auto-Compaction. [Gerber][W2], [Optitex][O4]. | Add bounded region compaction and retain locks. Show old/new length and retain the better valid version. Do not shift pieces through defects, shade bands or fold edges. |
| **Swap / local reinsert** | A useful operator action; no current vendor-specific exact swap command verified in the material checked. | Select two pieces or two bundle groups, try exchanging neighbourhoods, then repair locally. Preserve sizes, handedness and bundle IDs. Show a preview and reject invalid transforms. |
| **Flip, reverse, and fine tilt** | Optitex documents rotate/flip; Gerber has grouped rotation/flip permissions. [Marker][O3], [lay limits][W3]. | Expose per-placement permitted reflection, 180-degree reversal and fine angle entry. Add ±0.5/1-degree shortcuts only inside allowed tilt. A pair cannot become two left pieces. Flip is not the same as rotation. |
| **Temporary overlap / tolerance** | Richpeace describes manual overlap/fine adjustment; Optitex O/26 exposes overlap controls. [Brochure][R4], [O/26][O2]. | Permit visible invalid draft moves if useful, but block approval/export until valid. Separate numeric epsilon, cut spacing, and explicitly authorized overlap in sacrificial allowance. Default to zero true cut-outline overlap. Never “fix” efficiency by widening validator tolerance. |
| **Split piece** | Gerber Easy Marking documents Split Piece. [Advanced tab][W4]. | Separate splitting a marker/plot into windows from changing the garment. Pattern split needs an approved seam/overlap and new cut geometry/demand. Keep outside automatic efficiency optimization; add a traceable Pattern operation later. |
| **Sections / splice lines / window cuts** | TUKAmark size sections; Gerber splice/bump rules; Optitex batch bump lines. [TUKA][T4], [Gerber settings][W5], [O/26][O2]. | Add editable, labelled section boundaries and a section report. Keep plot/cut windows distinct from physical lay sections. Add allowed splice zones and forbid a splice through a piece. Recalculate end loss. |
| **Multi-width trials** | Automarker compares size combinations and widths; production planning uses different material widths. [Automarker][S1]. | CLI already trials widths. Add UI comparison tied to eligible rolls and usable width. Compare total order metres/kg/cost and remainders, not marker efficiency alone. Narrower fabric can have a higher percentage and still be worse for the order. |
| **Marker library / lay-rule reuse** | AccuPlan stores/reuses successful markers; Gerber can reconstruct placement rules. [Library][G6], [settings][W5]. | Add searchable styles, revisions, ratio, width, fabric/rules, efficiency and approval. Exact compatible hits open directly; close hits warm-start. Invalidate cache on geometry/rule changes. Maintain provenance. |
| **Group / marry / repeated blocks** | Gerber uses marriage/block groups; ASSYST uses dynamic fusing blocks and quality zones. [2026 notes][G2], [Automarker][S1]. | Engine rules exist. Add visual multi-select/group/ungroup, copied arrangements and lock groups. Keep optional optimizer clusters separate from mandatory sewing/matching relationships. |
| **Matching workspace** | CLO texture/print matching; Gemini visualizes checks against real fabric. [CLO][C1], [Gemini][M1]. | Add fabric repeat origin, named anchors, matched-part links, residual error in mm and a fabric-image overlay. Show which rule prevents a move. Keep shrinkage transformation and repeat coordinates consistent. |
| **Piece tray / counts / keyboard** | CLO has nudge and neighbouring-piece selection; commercial preparation depends on correct ratios. [CLO][C1]. | Visible remaining/placed/required counts by size and handedness. Search pieces. Group selection, numeric coordinates, measure, zoom-to-error, configurable shortcuts and undo/redo across all marker actions. |
| **Preview and compare** | Commercial systems let operators improve an existing marker and select better results. [AccuNest guide][G3]. | Keep approved, current and best-valid checkpoints. Display length saving in mm/% and efficiency in pp, plus validation status. Never replace a better valid marker with a worse returned run. |

### 5.2 Reports for Indian cutting rooms and merchandisers

There is no single verified universal “Indian consumption report” standard. A useful older local primary reference is [NITRA's 2019–20 garment-software description][W6]: buyer/job-wise fabric reconciliation, planned versus actual cutting average, utilization, cut quantity and balance. The layouts below are proposed templates, to be fitted to each buyer/factory.

| Document | Required fields and units | Delivery |
|---|---|---|
| **Style costing / consumption sheet** | Buyer, style/revision, season, PO, colour, fabric code/composition, width including usable width, GSM, shrinkage, size ratio and total garments. Marker length, efficiency, net m/garment, gross m/garment, purchase allowance and ₹/garment. Separate shell, lining, pocketing and fusing. Optional m/dozen, yd/garment and kg/garment. | Editable **XLSX**, clean CSV for systems, A4 PDF for approval. Explicit unit headings, date, preparer and approver. |
| **Width / ratio comparison** | Same order/rules; width, ratio, marker length, efficiency, required plies/lays, over-cut, total consumption, end/splice loss, eligible stock and ₹ total. | Side-by-side sheet with selected option and reasons. Keep alternatives reproducible. |
| **Lay / cutting ticket** | PO/style/colour, marker ID/revision, table, fabric usable width, lay length, plies, ratio, size output, roll/lot/dye/shade, approved shrinkage, splice instructions and print/cutter profile. | A4 PDF plus QR lookup; mini-marker preview. Human-readable IDs are mandatory even when QR is present. |
| **Roll issue / end-bit register** | Roll identity, received/usable width and length, issued metres/kg, used length, returned usable ends, scrap, defects, splice loss and remaining balance. | XLSX/CSV; signed issue/return record. Keep shade/dye identity on returned ends. |
| **Fabric reconciliation / cutting average** | Buyer/PO/job/month; planned and actual garments; gross issued minus usable returns; actual m/good garment; net cut area; marker waste, end loss, splice/defect loss, re-cuts and unexplained variance. | Buyer-wise monthly XLSX plus job detail. Show both marker efficiency and total cutting yield with their denominators. |
| **Bundle / re-cut ledger** | Lay/ply range, size, exact part/handedness, bundle, shade, intended line, cut/issued/received/re-cut quantities and damage reason. | Thermal label, A4 tickets, CSV/JSON export and scan history. Reprints carry the same identity plus a reprint audit. |

Use actual `length_mm / garments_in_marker` for net marker metres per garment after conversion to metres. Gross planned consumption also includes plies, repeats, end allowances, splice loss and required over-cut. Actual consumption uses measured issue/returns and good output. These three numbers must not share one ambiguous “average” field.

For plain open-width fabric, `kg = length_m × usable_width_m × GSM / 1000`. For folded/tubular stock, derive physical area from the stock's width convention and layers first. Do not multiply by two twice. GSM and width variation make this an estimate unless actual issue weight is recorded. Convert metres/yards exactly; label reporting rounding separately from geometry precision.

### 5.3 ERP integration order

| Connection | v0.4 scope | Fact behind the choice |
|---|---|---|
| **Excel / CSV first** | Versioned order, roll and actuals templates; column mapping, units, preview, row errors and idempotent external IDs. Export consumption and approved cut plans. Continue offline with a pending-outbox directory. | Existing BETA CSV/XLSX foundations make this the smallest adoption step. It also serves factories with custom ERPs. |
| **TallyPrime** | Start with mapped stock-item/roll references and reviewed consumption/stock-transfer exchange. Generate/import the site's accepted XML or Excel template. Optional local connector later. Use ODBC for supported reads/report extraction. | Tally documents XML/JSON/ODBC and an API explorer; current formats vary by release. Do not market a generic REST API as if every factory has it. [Integration][W7], [API explorer][W8]. |
| **SAP** | Map external PO/order, material, plant, location, batch, UoM and cut-job IDs. Prototype read-only order intake first. Add reviewed goods issue/return posting with retries and duplicate protection as a separately scoped adapter. | S/4HANA documents production-order read APIs and material-document POST APIs. Authentication, available API version and factory transaction rules remain site-specific. [Order API][W9], [material documents][W10]. |
| **Other ERP / PLM** | Publish JSON/CSV contracts, monitored import/export folders and completion acknowledgements. A later HTTP connector can use the same records. Store consumption as a revisioned result, not an overwrite of the only BOM value. | Valia and ET explicitly sell connected data flow. BETA can provide it without copying proprietary connectors. [Valia][L6], [ET][E2]. |

Do not write direct SQL into ERP tables. A cutting-room planning event is not necessarily an accounting posting. Map issue, return and scrap semantics with the factory, then test replay and reversal. This is implementation work, not evidence that BETA already integrates with Tally or SAP.

## 6. Prioritized v0.4 backlog: top 15

**E rank** = expected impact on valid marker efficiency. **A rank** = expected impact on factory adoption. 1 is highest. These are engineering judgments from local gaps and public evidence, not measured rankings. Overall order also considers dependencies and risk. Effort is **engineer-weeks**, with testing and documentation; ranges overlap and are not additive savings.

| Priority | Concrete change | E rank | A rank | Effort | Acceptance evidence |
|---:|---|---:|---:|---:|---|
| **1** | **Freeze a production comparison suite and approval gate.** Pinned geometry, rules, clocks and machines; independent validity and physical-area accounting. Include 151+ pieces, folds, bands, defects, matching, real vendor markers and output profiles. | 6 | 7 | **1–2** | Every result has source revision, hardware/load, seed, worker/CPU budget, time-to-best, length and validity. Hold out new styles. No saving claim based only on the Gemini example or an ambiguous sheet code. |
| **2** | **Replace construction-only folded/obstacle search with constrained separation and compaction.** Edge pieces slide along folds; free pieces can move in 2D; preserve fixed transforms and layer counts. | 1 | 8 | **4–6** | Target: at least **10% shorter** than current folded Teagan at the same 60 s, plus improvement on held-out folded styles. Treat 10% as a hypothesis. Zero invalid layouts; retain incumbent if repair fails. |
| **3** | **Search bundle-to-shade-band assignments.** Swap/rebalance whole bundles, jointly optimize lane lengths, warm-start affected sections; keep measured boundaries. | 2 | 9 | **3–5** | Target: recover **2 pp** on each local shade fixture without relaxing rules; report irreducible versus algorithmic loss on small exact/hand-checked cases. Validate on new styles before shipping claims. |
| **4** | **Build the fast operator toolkit.** Multi-select, contact bump in four directions, group compact, swap/reinsert, allowed flip/fine tilt, numeric placement, piece tray and unified undo. | 5 | 2 | **3–4** | Pilot operators complete ten specified rescue tasks with counts/rules intact; aim for **30% less task time** than current UI. Record failures as well as successes. Invalid drafts cannot be approved. |
| **5** | **Ship Tauri 2 desktop with durable projects.** Offline NSIS installer, native worker lifecycle, file associations, autosave/recovery and local library/queue database. | 15 | 1 | **2–3** | Clean Windows 10/11 x64 install with network disabled, including a PC missing WebView2. Open a file by double-click, cancel a job, force-close/recover, upgrade and reopen projects without loss. |
| **6** | **Add tail/pocket destroy-and-repair plus final compaction.** Remove selected small pieces or local regions; try several reinsertion orders, then validate. | 3 | 10 | **3–5** | Target: **0.3 pp mean** improvement on eligible held-out jobs at equal total time, with no degradation of the retained result. Report large-marker tail gains separately. |
| **7** | **Add reusable pair/cluster candidates.** Several arrangements per legal piece/size/orientation signature; repeat where useful and allow breakup. | 4 | 12 | **3–4** | Target: **0.5 pp** on the repeated-style subset at equal time. Compare against unclustered runs and disclose mixed-style regressions; no forced geometry or grain change. |
| **8** | **Marker library and validated warm reuse.** Search ratio/width/style/revision; exact reuse, similar-layout warm start, approved/best checkpoints. | 7 | 3 | **2–3** | Exact matches open without nesting; changed geometry/rules cannot silently reuse approval. Target: **50% less preparation time** for repeat orders in the pilot. |
| **9** | **Join width/ratio planning to real roll cost.** Expose existing batch widths; nest finalists longer; recompute final lay/roll allocation and end loss. | 8 | 5 | **3–4** | Choose by total feasible order cost/consumption, not highest density. Shortlisted plans use actual final marker lengths; allocation validates against roll, shade, shrinkage and demand. |
| **10** | **Certify plot/print/cut profiles.** Production PDF/HPGL spool paths, cutter profiles, section annotations, label layouts, retry and print history. | 14 | 4 | **2–4** | Factory-approved 1,000 mm calibration, long marker, notches/drills, lay ticket and scanned QR on each supported pilot device. A file download alone is not a passing test. |
| **11** | **Persistent unattended queue and time tiers.** Draft/costing/production/overnight, priority, pause/cancel/resume, bounded CPU, best-valid checkpoints and completion status. | 10 | 6 | **2–3** | Queue survives restart; failed jobs isolate errors; cancel yields best valid result. Publish 60 s/300 s/long-run curves and total occupied CPU. Existing CLI batch is the starting point. |
| **12** | **Merchandiser reports and exchange contracts.** Costing, lay ticket, width comparison, roll/end-bit register and buyer/job reconciliation in XLSX/CSV/PDF; external IDs and import preview. | 13 | 11 | **2–3** | Factory signs off two real PO report packs. Net/gross/actual consumption and m/kg/yard units reconcile. Tally/SAP adapters remain explicit follow-on scope, not a checkbox claim. |
| **13** | **Harden import and rules review.** DXF-AAMA/ASTM/RUL/vendor-PLT cases, unit preview, outline/seam choice, notches, grain, handedness and demand mapping; report unsupported semantics. | 12 | 13 | **2–3** | Round-trip fixtures plus at least three suppliers' consented examples. No silent missing piece, closed hole, lost grade or scaling mismatch. Operator sees warnings before nesting. |
| **14** | **Profile CPU search and adapt budgets.** Measure geometry conversion, collision, validation, repair and thread pools; tune phase time and stagnation restarts by job class. | 9 | 14 | **2–3** | Keep only changes improving held-out time-to-quality on 4/8/16 logical CPU profiles. No forecast based on Apple SIMD results. Cold start, memory, UI latency and end-to-end time reported too. |
| **15** | **Gate a genuinely diverse second search.** PackingSolver initializer or local-region solver; compare different policies with/without migration. Keep the current solver available. | 11 | 15 | **3–5** | Equal-total-thread/time ablation across held-out styles. Ship only a demonstrated benefit and licence-clean integration. Random seed count alone does not satisfy this item. |

### 6.1 Suggested release cut

Items 1–5 are the main release decisions. Start the benchmark gate first. Develop the desktop/editor workflow while the constrained search work runs. Then add local repair and clusters behind switches. Library, planning and production output should use the same approved marker record and revision hashes.

All 15 amount to roughly **37–57 engineer-weeks** before factory scheduling and contingencies. This is a programme, not a single short sprint. If the calendar is tight, ship a smaller v0.4 containing the desktop workflow, measured folded/shade fixes and certified outputs. Leave learning/GPU work as separately funded experiments.

### 6.2 Quality gates and honest targets

| Gate | Rule |
|---|---|
| Validity | Exact full demand, pair/handedness, grain/nap, folds, shrinkage, gap, defect, shade and matching checks on every approved result. Search may overlap temporarily; production output may not. |
| Efficiency | Proposed broad target: reduce the recorded 60 s mean deficit from 2.3556 pp toward **1 pp**, and matched-duration deficit from 1.0865 pp toward **0.5 pp**. These are release goals, not forecasts. Confirm sheet semantics before strengthening commercial claims. |
| Experiment design | Use paired jobs, predeclared seeds, fixed hardware/thread allocations and repeated runs. Keep tuning and hold-out sets separate. Do not sum technique gains. Report per-class mean/median/tails, win/tie/loss and invalid counts. |
| Time | Report both search time and end-to-end import/construction/search/repair/validation/export time. Use a real deadline and retain the best valid checkpoint. Queue throughput and one-job quality are separate measurements. |
| Resource fairness | Equal wall time and declared total thread/CPU budget. Control background work. Compare 1/3/5 seeds only when workers and coordinators fit the same budget. |
| Factory adoption | A repeat order must travel from imported patterns/order/rolls to approved marker, calibrated plot, cut ticket, bundles and reconciled report without developer intervention or an internet connection. |
| Geometry and licences | Keep original/effective cut geometry traceable. Include licence notices and source obligations for covered dependencies. Do not copy code or weights whose reuse terms are not established. |

## 7. Source register

All links were checked during this research on 19 October 2026. Older documents are labelled above. Where a page was searchable but a later fetch timed out or required login, only accessible text was used. Public claims and prices can change.

### Commercial sources

Key starting points: [SMARTmark production time guidance][T3], [TUKAcad prices][T2], [AccuMark 2026.1 notes][G2], [Optitex O/26][O2], [Valia 2026 workflow][L6], [ASSYST Automarker][S1], [CLO nesting manual][C1]. Detailed citations throughout the report resolve to the URL definitions below in the Markdown source.

[T1]: https://tukatech.com/products/smartmark "Tukatech SMARTmark: current product page"
[T2]: https://tukatech.com/products/tukacad "Tukatech TUKAcad: packages and prices"
[T3]: https://tukatech.com/news/best-roi-smartmark-queue-and-netq "Tukatech SMARTmark queue guidance, 25 March 2025"
[T4]: https://tukatech.com/news/easy-cad-marker-making-p3 "Tukatech: sectional markers and bump lines, legacy tutorial"
[T5]: https://tukatech.com/news/the-biggest-disruption-of-my-adult-life "Tukatech: GC and connected workflow, 18 May 2026"
[T6]: https://www.linkedin.com/posts/tukatech-inc_tukacadgarmentcreator-tukacad2026-tukatech-activity-7462200351006285824-RpW3 "Tukatech official post: TUX information and Garment Creator"
[T7]: https://www.linkedin.com/posts/tukatech_tukacad-fashiontech-tukatech-activity-7458382340352790528-rx11 "Tukatech founder demonstration: base-size placement across sizes"
[G1]: https://help.gerbertechnology.com/Whats_New/AccuMark/SupportedVersions.htm "AccuMark public release history, 2025–2026"
[G2]: https://help.gerbertechnology.com/Resources/v2026.1.relnot_PE.pdf "AccuMark Professional 2026.1 release notes, sections 2.11–2.15 and fixes"
[G3]: https://gerber-help.lectra.com/AccuNest/Getting_Started_with_AccuNest.htm "Gerber: AccuNest modes, editions and workflow"
[G4]: https://www.lectra.com/en/fashion/products/gerber-accumark-fashion "Lectra: current Gerber AccuMark product and subscription"
[G5]: https://gerber-help.lectra.com/AccuPlan/Lesson_3.htm "Gerber: AccuPlan workflow"
[G6]: https://gerber-help.lectra.com/AccuPlan/Lesson_1.htm "Gerber: AccuPlan marker library and reuse"
[G7]: https://www.lectra.com/sites/default/files/2023-03/solution-brochure-accumark-en.pdf "Lectra: AccuMark suite brochure, older source"
[L1]: https://www.lectra.com/sites/default/files/2021-02/pr-flex-offer-by-lectra-en_VF%20%281%29.pdf "Lectra: Flex Offer launch specification, 2021"
[L2]: https://www.lectra.com/en/library/united-creations-hits-new-production-targets-with-cloud-based-automated-nesting "Lectra: United Creations Flex Offer case, October 2024"
[L3]: https://www.lectra.com/en/library/fabric-estimation-reinvented-unmatched-accuracy-through-innovation "Lectra: Valia Estimate, June 2025"
[L4]: https://www.lectra.com/en/library/valia-fashion-the-5-essential-features-transforming-fashion-manufacturing "Lectra: Valia features, April 2026"
[L5]: https://www.lectra.com/en/library/how-valia-fashion-solves-the-material-waste-challenge "Lectra: AI nesting and motif-heavy waste claims, May 2026"
[L6]: https://www.lectra.com/en/library/discover-main-features-of-valia-fashion-fashion-manufacturing-solution-transforming-cutting "Lectra: Valia ERP, planning and equipment workflow, April 2026"
[O1]: https://optitex.com/page/2/?term=news-gated "Optitex: O/25 enhanced Nest++ Pro announcement, April 2025"
[O2]: https://optitex.com/optitex-o26-release/ "Optitex: O/26 nesting, batch bump/group/shade and PDF/DXF changes"
[O3]: https://optitex.com/products/marker/ "Optitex: multicore Marker and manual operations"
[O4]: https://exhibitorsearch.messefrankfurt.com/images/original/document_downloads/10000009202601/506383/1771345033642_2638381955.pdf "Optitex exhibitor brochure: Nest++ Pro, Match++, parallel nesting and compaction"
[A1]: https://audaces.com/en/solutions/supera "Audaces: Supera algorithms, queue and up-to-3% claim"
[A2]: https://audaces.com/en/faq "Audaces: DXF, rules, server and ICF questions"
[A3]: https://audaces.com/en/blog/discontinuation-notice-audaces-supera "Audaces: old coprocessor lifecycle and Supera Max"
[A4]: https://audaces.com/it/piani/aziende "Audaces: company bundles, price on request"
[S1]: https://style3d-assyst.com/automarker/ "ASSYST: Automarker cloud, quality zones and production packages"
[S2]: https://help.style3d.com/studio/en/1f8f/45fd8 "Style3D: October 2025 V9.1 marker, QR and BOM changes"
[S3]: https://www.style3d.com/blog/style3d-studio-v10-0-from-ai3d-output-to-enterprise-value/ "Style3D: V10 AI arrangement, sewing and workflow claims, 2026"
[S4]: https://www.automarker.com/2010/terms_en.html "Automarker historical terms naming DXF/HPGL/ISO"
[S5]: https://help.style3d.com/studio/en/c4905/c2c8/32fa/ca2a "Style3D: current Marker and Super Marker manual"
[C1]: https://support.clo3d.com/hc/en-us/articles/115000565568-Nest-Patterns "CLO: current Nest Patterns manual with 2026.1 notes"
[C2]: https://developer.clo3d.com/list.html "CLO: API list, export and nesting functions"
[C3]: https://support.clo3d.com/hc/en-us/articles/115012815487-How-much-does-CLO-cost "CLO: prices, updated September 2026"
[R1]: https://www.richpeace.com/garment_cad-show-1147.html "Richpeace: New Super Nest V10"
[R2]: https://www.richpeace.com/garment_cad-show-437.html "Richpeace: Garment CAD V10, formats and GMS"
[R3]: https://www.richpeace.com/uploadfile/202210/5085cf1486261f.pdf "Richpeace: older official design/grading/marking manual"
[R4]: https://download.richpeace.com/EN/2024/202401/2022.4.19%20CAD%20SOFTWARE%20V.9%281%29.pdf "Richpeace: V9 brochure, manual overlap and reverse duplication"
[B1]: https://en.bokecad.com/super-nesting/ "BOK: advertised V22 AI and four-core package prices"
[E1]: https://www.etsystem.cn/ "ETSYSTEM official site and file conversion entry"
[E2]: https://www.etsystem.cn/News/detail.html?id=1122 "ETSYSTEM: cutting workflow and standard data APIs, November 2021"
[E3]: https://www.etsystem.cn/News/detail.html?id=1163 "ETSYSTEM: automatic nesting and interfaces review, January 2023"
[M1]: https://geminicad.com/cutting-room/ "Gemini: current cutting-room workflow and constraints"
[M2]: https://cdn.geminicad.com/wp-content/uploads/2018/09/DL%20-%20Nest%20Expert%20websize.pdf "Gemini: Nest Expert multicore/server, formats and restrictions, older sheet"
[M4]: https://www.linkedin.com/posts/gemini-cad-systems_geminicad-nestexpert-fabricsavings-activity-7076551077126451201-7qoD "Gemini official older post: 2–10% and 27% claims"
[N1]: https://gelianfashion.com/ "Gelian: Marka and Ultranest product claims"
[N2]: https://fashioninsta.ai/ "FashionInsta: customer DXF archive and Pattern Nesting proposition"
[N3]: https://www.sixatomic.com/synthesis "Six Atomic: Synthesis pattern automation"
[N4]: https://deepnest.io/ "Deepnest: part-in-part, formats and desktop deployment"

### Research and code sources

Start with the [Sparrow paper][P1], [Jagua 1.1 changes][J3], [GFPack++ paper][F1], [GFPack++ code/licence status][F3], and [PackingSolver][Q1].

[P1]: https://arxiv.org/html/2509.13329v4 "Gardeyn et al.: Sparrow paper, v4, 6 October 2026"
[P2]: https://github.com/JeroenGar/sparrow "Sparrow implementation, MIT, usage and integration"
[P3]: https://github.com/JeroenGar/sparrow/releases/tag/v0.2.0 "Sparrow 0.2 release: paired throughput evidence"
[P4]: https://github.com/JeroenGar/sparrow/releases/tag/v0.3.0 "Sparrow 0.3 release: schema and dependency changes"
[P5]: https://doi.org/10.1016/j.ejor.2013.06.020 "Elkeran: guided cuckoo search and pairwise clustering, 2013"
[P6]: https://www.sciencedirect.com/science/article/abs/pii/S2405896318314216 "Pairwise clustering using the dotted board model, 2018"
[P7]: https://www.sciencedirect.com/science/article/abs/pii/S2405896315003675 "Multiresolution overlap minimization and GPU raster potential, 2015"
[P8]: https://doi.org/10.1016/j.ejor.2022.03.034 "Umetani and Murakami: coordinate descent for rasterized shapes, 2022"
[P9]: https://www.euro-online.org/websites/esicup/wp-content/uploads/sites/12/2025/04/ESICUP21st-4.pdf "ESICUP 2025: 3D voxel GPU strip-packing abstract"
[J1]: https://arxiv.org/html/2508.08341v4 "Gardeyn et al.: Jagua collision engine manuscript"
[J2]: https://github.com/JeroenGar/jagua-rs "Jagua implementation and MPL-2.0 licence"
[J3]: https://github.com/JeroenGar/jagua-rs/releases/tag/v1.1.0 "Jagua 1.1: generic hazards, quality zones and simplification"
[F1]: https://arxiv.org/html/2406.07579v2 "GFPack++: ICCV version, updated October 2026; table 1 and supplement"
[F2]: https://openaccess.thecvf.com/content/ICCV2025/html/Xue_GFPack_Attention-Driven_Gradient_Fields_for_Optimizing_2D_Irregular_Packing_ICCV_2025_paper.html "GFPack++: ICCV 2025 proceedings"
[F3]: https://github.com/TimHsue/GFPack-pp "GFPack++ code; no root licence verified"
[F4]: https://link.springer.com/article/10.1007/s10845-025-02620-6 "Abdou et al.: DRL rotational placement, online 2025 / volume 2026"
[F5]: https://link.springer.com/article/10.1007/s10479-025-06506-x "Jiang et al.: incremental SVR-assisted multi-objective search, 2025/2026"
[I1]: https://onlinelibrary.wiley.com/doi/10.1155/2017/1670709 "Amaro Junior et al.: parallel multiple-population BRKGA, 2017"
[I2]: https://www.mdpi.com/2227-7390/14/8/1301 "Liu et al.: modular island metaheuristic, April 2026"
[I3]: https://www.kaggle.com/competitions/santa-2025/writeups/4th-place-hybrid-packing-with-sparrow-manual-str "Santa 2025 fourth-place structural hybrid solution, February 2026"
[Q1]: https://github.com/fontanf/packingsolver "PackingSolver: MIT and available engines"
[Q2]: https://fontanf.github.io/packingsolver/irregular.html "PackingSolver irregular API"
[Q3]: https://github.com/TexasCoding/ironnest "ironnest: MPL-2.0 fork and unverified performance claims"
[Q4]: https://arxiv.org/abs/2503.21009 "Lastra-Diaz and Ortuno: parallel exact discrete-grid nesting, 2025"

### Desktop and workflow sources

Start with [Tauri Windows packaging][D6], [Microsoft WebView2 distribution][D7], [Slint's exact royalty-free terms][D5], [Gerber bump/slide operations][W1], [Tally integration][W7] and [SAP material documents][W10].

[D1]: https://github.com/tauri-apps/tauri "Tauri: architecture and MIT/Apache-2.0 licences"
[D2]: https://raw.githubusercontent.com/emilk/egui/master/Cargo.toml "egui licence declaration"
[D3]: https://github.com/iced-rs/iced "iced: Rust GUI and MIT licence"
[D4]: https://slint.dev/faqs "Slint: GPL, royalty-free and paid alternatives"
[D5]: https://raw.githubusercontent.com/slint-ui/slint/master/LICENSES/LicenseRef-Slint-Royalty-free-2.0.md "Slint royalty-free licence 2.0, exact terms"
[D6]: https://v2.tauri.app/distribute/windows-installer/ "Tauri 2: NSIS/MSI and offline WebView2 packaging"
[D7]: https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution "Microsoft: deploy Evergreen or fixed WebView2, including offline"
[D8]: https://support.microsoft.com/en-us/windows/deployment/updates-lifecycle/windows-10-support-has-ended-on-october-14-2025 "Microsoft: Windows 10 support lifecycle"
[D9]: https://learn.microsoft.com/en-us/deployedge/microsoft-edge-supported-operating-systems "Microsoft: WebView2/Edge support on Windows 10 22H2 through at least October 2028"
[D10]: https://v2.tauri.app/reference/config/ "Tauri: bundle/file-association configuration"
[D11]: https://docs.rs/tauri/latest/tauri/webview/struct.WebviewWindow.html#method.print "Tauri: webview print method"
[W1]: https://help.gerbertechnology.com/AccuMark/Easy_Marking/Group_Bumpline_Tab_Fabric.htm "Gerber: bump lines and sliding"
[W2]: https://help.gerbertechnology.com/AccuMark/Easy_Marking/Tab_Marker.htm "Gerber: Compact Marker and marker operations"
[W3]: https://gerber-help.lectra.com/AccuNest/Group_Function_in_Lay_Limits_for_AccuNest_Placement.htm "Gerber: grouped flip/rotation permissions"
[W4]: https://gerber-help.lectra.com/AccuMark/Easy_Marking/Tab_Advanced.htm "Gerber: Split Piece and advanced marker operations"
[W5]: https://help.gerbertechnology.com/AccuMark/Easy_Marking/Group_Display_Tab_View.htm "Gerber: lay-rule reuse, float/step, splices, buffers and block fusing"
[W6]: https://www.nitratextile.org/pdf/annual19920.pdf "NITRA 2019–20 annual report: garment job and fabric reconciliation software"
[W7]: https://tallysolutions.com/integration/ "TallyPrime: supported integration approaches"
[W8]: https://tallysolutions.com/tallyprime-api-explorer/ "TallyPrime: XML/JSON API explorer and test environment"
[W9]: https://help.sap.com/docs/SAP_S4HANA_ON-PREMISE/a6f0333202384ba2b48a841a4a6deb1b/93b5355889729344e10000000a4450e5.html "SAP: production-order-item read API, 2025 FPS01"
[W10]: https://help.sap.com/docs/SAP_S4HANA_ON-PREMISE/eb2a39dd0c124fed8252f684002d55e1/8bb0d08295044ee3af444b4f2a6e4457.html "SAP: material-document creation API, 2025 FPS01"
