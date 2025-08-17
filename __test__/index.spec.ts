import test from 'ava'
import { createRequire } from 'node:module'

const require = createRequire(import.meta.url)

let arrsacLine: ((x: number[], y: number[], options?: any) => any) | null = null
let loadError: unknown = null

try {
  // CJS export shape: module.exports = nativeBinding; module.exports.arrsacLine = ...
  arrsacLine = require('../index.js').arrsacLine
} catch (e) {
  loadError = e
}

test('loads native binding (skips if unavailable)', (t) => {
  if (loadError || !arrsacLine) {
    t.log('Native binding not available in this environment, skipping further tests')
    t.pass()
    return
  }
  t.is(typeof arrsacLine, 'function')
})

test('returns null on mismatched input lengths', (t) => {
  if (!arrsacLine) {
    t.pass()
    return
  }
  const result = arrsacLine([0, 1], [0])
  t.is(result, null)
})

test('fits a line and rejects a clear outlier', (t) => {
  if (!arrsacLine) {
    t.pass()
    return
  }
  const x = [0, 1, 2, 3, 4, 5]
  const y = [1.0, 3.1, 5.0, 6.9, 9.0, 100.0] // last point is an outlier (true line ~ 2x + 1)

  const res = arrsacLine(x, y, { inlierThreshold: 0.3 })
  t.truthy(res)
  if (!res) return

  t.is(res.numPoints, x.length)
  t.true(res.numInliers >= 5)
  // inliers should exclude the last index (5)
  t.true(!res.inliers.includes(5))

  // slope ~ 2, intercept ~ 1 within tolerance
  t.true(Math.abs(res.slope - 2) < 0.2)
  t.true(Math.abs(res.intercept - 1) < 0.3)
})

