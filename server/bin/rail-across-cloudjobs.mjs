#!/usr/bin/env node
// rail-across-cloudjobs — prepares uploaded point clouds (phase 13, AP 13.3) and runs
// the long checks over them (AP 13.7).
//
//   rail-across-cloudjobs serve        take jobs from the queues, at most two of each at once
//   rail-across-cloudjobs run <job>    prepare one cloud (what serve starts per job)
//   rail-across-cloudjobs exec <run>   one long run (what serve starts per run)
//
// Same database and cloud directory as rail-across-server (RA_SERVER_DB,
// RA_SERVER_CLOUDS). The jobs run the browser's own point cloud code, the
// package @rail-across/pointcloud.
const { main } = await import('../src/clouds/cli.js')
await main(process.argv.slice(2))
