# Frequently asked questions

These answers cover public 0.7 and clearly marked unreleased 0.8 work.
See [features](FEATURES.md) for screen locations and [getting started](GETTING-STARTED.md)
for short tasks. The first eight answers are the proposed shared website FAQ.

## 1. Is it free?

Yes. BETA's Apache-2.0 code has no licence fee. You still pay for hardware,
hosting, printing and any help you arrange. See [licence notices](../THIRD-PARTY-LICENSES.md).

## 2. Is it really open source?

Yes. You can inspect, build and change the code. Third-party code and data
have separate terms; keep their notices. See [credits](../README.md#credits-and-licences).

## 3. Who built it, and how was AI used?

Sohit and contributors built it with help from modern AI tools. It is a fun,
young project. It needs real users and independent review. The repository
history records contributions; see [how to help](public-root/CONTRIBUTING.md).

## 4. Can I trust software built with AI?

Judge the checks and results, not the tool used to write it. Tests and an
independent marker validator help, but do not prove fit or machine safety.
Check scale, fit, layer counts and output before production. See [release limits](RELEASE-0.7.0.md).

## 5. Does it replace Gerber, Lectra, Tukatech, Optitex or Gemini?

That is not established. BETA covers useful workflows but lacks commercial
CAD breadth, certified exchange and proven hardware support. There is no
general superiority claim. See [status](CUTTING-ROOM-STATUS.md) and [benchmarks](benchmarks.md).

## 6. Where does my factory data go?

Static browser work stays in the browser profile; desktop work stays on the PC.
If you choose a server, requests go there. There is no analytics or automatic
upload. Public feedback attachments become public. See [privacy](PRIVACY.md).

## 7. Does it work offline?

The browser can work offline after its first successful load and cache. Test
this before relying on it. Desktop computation is local. Save file backups;
browser storage can be cleared or evicted. See [installation](INSTALL.md).

## 8. How good is the nesting, in numbers?

Historical native Lectra sample: at 60 s, 300 tasks, mean -0.73 percentage
points, median -1.22, 84 wins / 12 ties / 204 losses. At recorded duration on
60 tasks: mean +0.42, median -0.30, 25 / 3 / 32. These are not 0.8 rerun or
phone results. See the [method and caveats](benchmarks.md).

## 9. What is the business model?

The current project offers open-source code without a licence fee. No paid
support commitment or future funding plan is promised here. Ask the maintainer
before depending on a service contract; see [support](#30-is-there-support).

## 10. May I use it commercially or change it for my factory?

Yes, under Apache-2.0 and the applicable dependency/data terms. Preserve required
notices when distributing copies. The licence provides no warranty. See
[third-party licences](../THIRD-PARTY-LICENSES.md).

## 11. Does it need an account or a server?

No account. The static browser edition runs patterns, nesting, cutting-room
planning and exports locally. Only the separate board-photo digitizer needs
desktop/server. See [browser coverage](BROWSER-CUTROOM-API.md).

## 12. Which pattern files can I import?

Supported DXF-AAMA/ASTM entities, optional bounded ASCII RUL grading and a
Seamly2D subset. Not every vendor dialect is supported. Review units, counts,
pairs, marks and warnings. See [DXF coverage](BROWSER-EXPORT-API.md).

## 13. Can I use PLT/HPGL?

Yes, for plotting output and PLT reference import in Compare. A plot file does
not retain all editable pattern semantics. Check scale and pen behaviour on
your device. See [exports](BROWSER-EXPORT-API.md).

## 14. What about ISO cutter files and my cutter?

Generic ISO output exists. It is not certification for a particular cutter.
Review the machine profile and do an operator-approved dry run. Vendor cutter
formats are not promised. See [installation checks](INSTALL.md#before-cutting-cloth).

## 15. Can I print full size on A4, A3 or a roll plotter?

Yes. PDF supports full-size and tiled output; PLT/HPGL serves compatible plotters.
Print at actual size, turn off fit-to-page and measure the calibration marks.
See [browser exports](BROWSER-EXPORT-API.md).

## 16. Can it import patterns from a PDF?

There is no general PDF pattern importer. PDF is an output. Use a supported
DXF or the reviewed paper-photo workflow instead; see [features](FEATURES.md).

## 17. How accurate is phone photo digitizing?

Real factory-photo accuracy has not been established. Keep the A4 reference
flat beside the paper and check known distances. Perspective, curl and lens
distortion can cause errors. One passing distance does not validate every
curve. See [paper-pattern limits](PAPER-PATTERNS-MSME-2026-10-26.md).

## 18. Can photos make all my sizes automatically?

No. Photograph each physical size in the paper workflow. The separate Style
workflow grades from charts/rules; photos do not recover those rules. See
[getting started](GETTING-STARTED.md).

## 19. Can I save a paper-pattern library?

Experimental 0.8 saves one reusable style set with outlines and fabric profiles.
It excludes source photos and restarts review/tape checks on reopening. It is
not a searchable multi-style library. See [paper sets](PAPER-PATTERNS-MSME-2026-10-26.md).

## 20. Does it support folded and tube knitwear?

Yes, with physical-layer allocation and useful-yield reporting. Small odd
orders can still use substantially more fabric than open lays. Compare metres
per garment, surplus and layer counts. See [fold results](../crates/beta-marker-nest/FOLD7_RESULTS.md).

## 21. What is a ply, a pass and a lay?

A ply is one fabric layer. A pass lays one length from the roll: one ply open,
two folded/tube. A lay is the stack for a marker. See [examples and file versions](PLIES.md).

## 22. Can it handle stripes, checks, nap and shrinkage?

Yes, within supported repeat/match rules and measured shrinkage inputs. Inspect
the overlay and rule report. GSM does not infer shrinkage or stretch. See
[Style rules](../../pattern/docs/STYLE-API.md).

## 23. Are the blocks ready for production?

They are starting drafts. Shape/seam tests and measurement sheets do not prove
fit. Have a pattern master review and sew a sample. See [block checks](../../pattern/docs/BLOCK-CHECKS.md).

## 24. Does grading support factory rule tables?

Yes: chart grading, point rules, and reviewed JSON/bounded RUL libraries.
Universal vendor RUL compatibility and a finished-garment constraint solver
are not provided. See [production tools](../../pattern/docs/PRODUCTION-TOOLS.md).

## 25. Can it compare fabric widths and order cost?

Experimental 0.8 compares open-fabric width offers, ratio menus and optional
eligible stock using measured nesting. It is a bounded shortlist, not an exact
global optimum. Zero prices are not quotations. See [workflow limits](WORKFLOW-IMPROVEMENTS-2026-10-25.md).

## 26. Does it support Hindi and Tamil?

Draft translations exist. They need native-speaking factory review, and some
new panels remain English. Please report unclear words with screen context;
see [contributing](public-root/CONTRIBUTING.md).

## 27. What about Mac, Linux and phones?

Use a compatible browser. Phones suit small tasks; large markers need a larger
screen and more memory. The packaged desktop is Windows x64; Linux is a server
route. No packaged Mac desktop is promised. See [installation](INSTALL.md).

## 28. Can I host it for my factory?

Yes. The Rust server defaults to loopback. Remote use needs a bearer token,
HTTPS proxy, firewall rules and backups. It has no per-user roles or shared
document library. See [self-hosting](SELF-HOSTING.md).

## 29. Does it connect to ERP or control my spreader?

It exchanges reviewed CSV/XLSX/JSON and generic spreading instructions. There
is no live Tally/SAP connector, machine tension/speed control or automatic
sewing receipt. See [module status](CUTTING-ROOM-STATUS.md).

## 30. Is there support?

Community issues and Discussions are the current routes; no response-time
guarantee is offered. Use the app's Feedback button or contact
sohitqwer@gmail.com. Share only cleared examples. See [contributing](public-root/CONTRIBUTING.md).

## 31. How do I report a bug or security issue?

For bugs, give the version, screen, steps and expected/actual result with a
synthetic input. For security, report privately as described in
[security policy](../SECURITY.md). Never post tokens or private patterns.

## 32. How can a non-developer help?

Review language, test print scale, time a trial or explain a confusing step.
Offer a permission-cleared example before uploading it publicly. See
[contribution options](public-root/CONTRIBUTING.md).

## 33. Can developers contribute?

Yes. Keep computation in Rust, test failure cases and preserve validation,
offline use and licence notices. See [contributing](public-root/CONTRIBUTING.md).

## 34. What is next?

Measured photo and hardware trials, folded/tube quality, safer release checks
and clearer daily work. The [roadmap](ROADMAP.md) separates current work from
ideas. It is not a delivery-date promise.

## 35. Does it train AI on my patterns?

No automatic pattern upload or training service is part of the app. AI-assisted
development is separate from using the software. Review [privacy](PRIVACY.md)
and your own browser/OS settings.

## 36. What changes in 0.8, and can I try it now?

The branch adds paper sets/tape checks, bounded area improvement, width/cost
comparison and nesting experiments. They are not yet in the public 0.7 app.
Follow [features](FEATURES.md) and [roadmap](ROADMAP.md); check Releases for
what is actually published.
