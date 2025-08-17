### arrsac

Robust line fitting for Node.js powered by a Rust implementation of ARR-SAC.

This package exposes a single function `arrsacLine(x, y, options)` that estimates a best-fit line while rejecting outliers, and returns rich quality metrics.

### Install

```bash
npm i arrsac
# or
yarn add arrsac
```

Prebuilt native binaries are provided for major platforms and architectures. No toolchains are required for installation under normal circumstances.

### Quick start

```ts
import { arrsacLine } from 'arrsac'

const x = [0, 1, 2, 3, 4, 5]
const y = [0.1, 1.0, 2.0, 2.9, 4.2, 20] // 20 is a clear outlier

const result = arrsacLine(x, y, { inlierThreshold: 0.5 })

if (result) {
  console.log('slope/intercept', result.slope, result.intercept)
  console.log('numInliers', result.numInliers, 'of', result.numPoints)
  console.log('R^2 (inliers)', result.r2Inliers)
  console.log('inlier indices', result.inliers) // 0-based indices
}
```

### API

```ts
arrsacLine(x: number[], y: number[], options?: ArrsacOptions | null): ArrsacResult | null
```

- Returns `null` when `x` and `y` have different lengths or fewer than 2 points.
- Inlier indices are 0-based and refer to positions in the input arrays.

#### ArrsacOptions

- `inlierThreshold` (number, default 0.3): Residual threshold used by ARR-SAC to classify inliers.
- `minSlope` (number | undefined): Reject candidate models with slope less than this value.
- `maxSlope` (number | undefined): Reject candidate models with slope greater than this value.
- `minIntercept` (number | undefined): Reject candidate models with intercept less than this value.
- `maxIntercept` (number | undefined): Reject candidate models with intercept greater than this value.

#### ArrsacResult

- `slope` (number): Estimated line slope.
- `intercept` (number): Estimated line intercept.
- `inliers` (number[]): 0-based indices of inlier points.
- `r2All` (number): R² over all points. May be `NaN` if degenerate.
- `r2Inliers` (number): R² over inliers only. `NaN` if fewer than 2 inliers.
- `rmseAll` (number): RMSE over all points. `NaN` if no points.
- `rmseInliers` (number): RMSE over inliers. `NaN` if no inliers.
- `madAll` (number): Median absolute deviation of residuals over all points.
- `madInliers` (number): MAD over inliers.
- `iqrAll` (number): Interquartile range of residuals over all points.
- `iqrInliers` (number): IQR over inliers.
- `inlierRatio` (number): `numInliers / numPoints`.
- `numInliers` (number)
- `numPoints` (number)
- `maxAbsResidual` (number): Maximum absolute residual across all points.
- `threshold` (number): The inlier threshold actually used.

### Notes and tips

- ARR-SAC is stochastic; this binding uses a fixed RNG seed for reproducibility.
- For nearly vertical lines, ARR-SAC returns an extremely large `slope` with a corresponding `intercept` to approximate verticality.
- Constraining `minSlope`/`maxSlope` and/or intercept bounds can reduce false positives in noisy domains.

### Platform support

Prebuilt binaries are published for:

- macOS (x64, arm64)
- Linux (x64 gnu/musl, arm/arm64 gnu/musl, riscv64 musl, ppc64, s390x)
- Windows (x64, arm64)
- FreeBSD (x64, arm64)

The loader will automatically pick the correct binary. If no suitable binary is found, loading will fail with a detailed error containing the attempted targets.

### Browser / WASI (experimental)

Experimental WASI support is wired into the loader. If you provide a WASI build (`arrsac-wasm32-wasi`) or `./arrsac.wasi.cjs` and set `NAPI_RS_FORCE_WASI=1`, the module will attempt to load the WASI variant. A helper worker script is included at `wasi-worker-browser.mjs` for integrating with `@napi-rs/wasm-runtime` in Web Workers.

This path is experimental and not published by default; you are expected to provide the WASI artifact yourself.

### Development

Requires recent Rust and Node.js.

```bash
git clone <this repo>
cd arrsac
yarn
yarn build   # build native addon in release mode
yarn test    # run tests
```

Benchmark script is available via `yarn bench`.

### License

MIT
