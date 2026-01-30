const repl = require('repl');
const { manifest, connection, cache, database } = require('./schema.js');

console.log('=== Conduct Engine REPL ===');
console.log('Available functions:');
console.log('  - manifest(keyPath)   : Read manifest YAML');
console.log('  - connection(keyPath) : Read connection YAML');
console.log('  - cache(keyPath)      : Read cache YAML');
console.log('  - database(keyPath)   : Read database YAML');
console.log('');
console.log('Examples:');
console.log('  connection("common")');
console.log('  cache("user")');
console.log('  database("users")');
console.log('');

const replServer = repl.start({
  prompt: 'conduct> ',
  ignoreUndefined: true
});

// グローバルコンテキストに関数を追加
replServer.context.manifest = manifest;
replServer.context.connection = connection;
replServer.context.cache = cache;
replServer.context.database = database;
