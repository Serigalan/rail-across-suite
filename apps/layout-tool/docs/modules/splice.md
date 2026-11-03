# splice — joining two track ends

`src/core/utils/splice/` · needs `rules`

What "Zwei Gleise zusammenfügen" can change where the rule catalogue rejects a curvature jump. The construction
itself — every pair of ends, the transitions in their modes, the judging and ranking of the solutions, what would
fit where nothing does, the spacing to another track — is the optimizer module's
([optimizer](optimizer.md), Rust, decision 0019 of the suite); the dialog's request and commit are in
`commands/splice.js`.

## Files

| File | What it does |
|---|---|
| `spliceRemedies.js` | What a splice can change when LP.KS.01 rejects a curvature jump: speed at the joint, a larger radius, transitions instead of the jump |

## Remedies

Where LP.KS.01 (comparison radius at a curvature jump) rejects a joint, `spliceRemedies` solves again with:
a lower speed at the joint (the highest that removes the error), a larger radius of the inserted arc (rounded
up), or transitions instead of the jump — each offered with what remains on it. The same remedies (speed,
transition before the arc) are offered when connecting a switch to a track (`commands/connectRemedies.js`).

## Tests

`spliceRemedies.test.js`, through the optimizer module (skipped where it has not been built).
