# Dataset and model attribution

Model weights in `lectra-v1.json`, aggregate evaluation summaries and the derived
feature-only golden checks are distributed under **CC BY 4.0**:
https://creativecommons.org/licenses/by/4.0/ .

Source: Corentin Lallier, Laurent Vézard, Bruno Pinaud and Guillaume Blin (2022),
*Nesting Tasks Dataset for 2D-Nesting Efficiency Estimation*, version 1.1,
Zenodo record 7030786, https://doi.org/10.5281/zenodo.7030786 .

Changes: restricted decoding and conversion to millimetres; rotation-invariant
shape statistics; copy-weighted task features; regression-tree fitting to recorded
efficiency; validation-residual calibration; conversion to numerical JSON trees
for independent Rust inference. No source geometry is included. The authors and
Lectra do not endorse this model or Project BETA.

Code in this crate is Apache-2.0, separate from the derived model/data licence.
