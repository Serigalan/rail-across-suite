#!/usr/bin/env node
// olt-cloudjobs — prepares uploaded point clouds (phase 13, AP 13.3) and runs
// the long checks over them (AP 13.7).
//
//   olt-cloudjobs serve        take jobs from the queues, at most two of each at once
//   olt-cloudjobs run <job>    prepare one cloud (what serve starts per job)
//   olt-cloudjobs exec <run>   one long run (what serve starts per run)
//
// Same database and cloud directory as olt-server (OLT_SERVER_DB,
// OLT_SERVER_CLOUDS). The jobs run the browser's own point cloud code, the
// package @rail-across/pointcloud.
const { main } = await import('../src/clouds/cli.js')
await main(process.argv.slice(2))
