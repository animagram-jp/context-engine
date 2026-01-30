// Test script for state-engine
const { manifest, connection, cache, database } = require('./schema.js');

console.log('=== Testing state-engine ===\n');

// Test 1: cache.user
console.log('1. cache("user"):');
try {
    const result = cache('user');
    console.log(JSON.stringify(result, null, 2));
} catch (e) {
    console.error('Error:', e.message);
}

console.log('\n2. cache("app"):');
try {
    const result = cache('app');
    console.log(JSON.stringify(result, null, 2));
} catch (e) {
    console.error('Error:', e.message);
}

console.log('\n3. connection("common"):');
try {
    const result = connection('common');
    console.log(JSON.stringify(result, null, 2));
} catch (e) {
    console.error('Error:', e.message);
}

console.log('\n4. connection("tenant"):');
try {
    const result = connection('tenant');
    console.log(JSON.stringify(result, null, 2));
} catch (e) {
    console.error('Error:', e.message);
}

console.log('\n5. database("users"):');
try {
    const result = database('users');
    console.log(JSON.stringify(result, null, 2));
} catch (e) {
    console.error('Error:', e.message);
}

console.log('\n=== Test completed ===');
