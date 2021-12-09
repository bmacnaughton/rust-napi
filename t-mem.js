'use strict';

const {Mem} = require('./test.linux-x64-gnu.node');

const N = 1000000;
class Mem2 {
  constructor(n) {
    this.buffer = Buffer.alloc(n * 8);
  }
}

let Ctor;
const how = process.argv[2];
if (how === 'rust') {
  Ctor = Mem;
} else if (how === 'node') {
  Ctor = Mem2;
} else {
  // eslint-disable-next-line no-console
  console.log('usage: node t-mem.js rust|node');
  process.exit(1);
}

// eslint-disable-next-line no-console
console.log(`executing with ${how} allocator`);

// execute both to set baseline initialization costs.
new Mem(N);
new Mem2(N);

let lastMemTime = Date.now();
let i = process.memoryUsage();
const initialRss = i.rss;
let count = 0;

setInterval(allocate, 1);

function allocate() {
  for (let i = 0; i < 2; i++) {
    const m = new Ctor(N);
    count += 1;
  }
  if (Date.now() - lastMemTime > 5000) {
    lastMemTime = Date.now();
    const m = process.memoryUsage();
    // eslint-disable-next-line no-console
    console.log(`${count} Δrss ${m.rss - initialRss} rss ${m.rss - i.rss} hu ${m.heapUsed - i.heapUsed} x ${m.external - i.external}`);
    i = m;
  }
}
