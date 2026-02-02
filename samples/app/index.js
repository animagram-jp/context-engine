/**
 * state-engine Sample Application
 *
 * Demonstrates state-engine concepts with actual DB/KVS connections.
 */

require('dotenv').config();
const yaml = require('js-yaml');
const fs = require('fs');
const path = require('path');

// Import adapters
const ProcessMemoryAdapter = require('./adapters/process_memory');
const ENVAdapter = require('./adapters/env_client');
const KVSAdapter = require('./adapters/kvs_client');
const DBAdapter = require('./adapters/db_client');

async function main() {
  console.log('=== state-engine Sample App ===\n');

  // 1. Load YAML manifests
  const manifestPath = path.join(__dirname, '..', 'manifest');
  console.log(`1. Loading manifests from: ${manifestPath}`);

  const connectionYaml = yaml.load(
    fs.readFileSync(path.join(manifestPath, 'connection.yml'), 'utf8')
  );
  const cacheYaml = yaml.load(
    fs.readFileSync(path.join(manifestPath, 'cache.yml'), 'utf8')
  );

  console.log('   - connection.yml loaded');
  console.log('   - cache.yml loaded\n');

  // 2. Setup adapters
  const processMemory = new ProcessMemoryAdapter();
  const envClient = new ENVAdapter();
  const kvsClient = new KVSAdapter();
  const dbClient = new DBAdapter();

  console.log('2. Adapters initialized\n');

  try {
    // 3. Test DB connection
    console.log('3. Testing database connection...');
    const dbConfig = {
      host: envClient.get('DB_HOST'),
      port: parseInt(envClient.get('DB_PORT')),
      database: envClient.get('DB_DATABASE'),
      username: envClient.get('DB_USERNAME'),
      password: envClient.get('DB_PASSWORD'),
    };
    console.log(`   Connecting to: ${dbConfig.host}:${dbConfig.port}/${dbConfig.database}`);

    const users = await dbClient.fetchAll(dbConfig, 'users');
    console.log(`   Found ${users.length} users in database`);
    console.log('');

    // 4. Set user context
    const testUser = users[0];
    processMemory.set('userkey.sso_user_id', testUser.sso_user_id);
    processMemory.set('userkey.tenant_id', testUser.tenant_id);

    console.log('4. User context set:');
    console.log(`   - sso_user_id: ${testUser.sso_user_id}`);
    console.log(`   - tenant_id: ${testUser.tenant_id}\n`);

    // 5. Simulate State::get("cache.user") with auto-load
    console.log('5. Simulating State::get("cache.user") with auto-load...\n');

    // 5-1. Check KVS (store)
    const userStoreKey = resolvePlaceholders(
      cacheYaml.user._store.key,
      { sso_user_id: testUser.sso_user_id }
    );
    console.log(`   Step 1: Check _store (KVS)`);
    console.log(`   Key: ${userStoreKey}`);

    let userData = await kvsClient.get(userStoreKey);
    console.log(`   Result: ${userData ? 'HIT' : 'MISS'}\n`);

    if (!userData) {
      // 5-2. Auto-load from DB
      console.log(`   Step 2: Auto-load from _load (DB)`);
      const loadConfig = cacheYaml.user._load;
      const whereClause = resolvePlaceholders(
        loadConfig.where,
        { sso_user_id: testUser.sso_user_id }
      );
      console.log(`   Table: ${loadConfig.table}`);
      console.log(`   Where: ${whereClause}`);

      const dbRow = await dbClient.fetchOne(dbConfig, loadConfig.table, whereClause);
      console.log(`   DB Result: ${dbRow ? 'Found' : 'Not found'}\n`);

      if (dbRow) {
        // 5-3. Map DB columns to cache keys
        userData = {};
        for (const [cacheKey, dbColumn] of Object.entries(loadConfig.map)) {
          userData[cacheKey] = dbRow[dbColumn];
        }

        console.log(`   Step 3: Save to _store (KVS)`);
        console.log(`   Data:`, userData);
        await kvsClient.set(userStoreKey, userData, cacheYaml.user._store.ttl);
        console.log(`   TTL: ${cacheYaml.user._store.ttl} seconds\n`);
      }
    }

    // 6. Verify cache hit
    console.log('6. Verify cache (should be HIT now)...');
    const cachedData = await kvsClient.get(userStoreKey);
    console.log(`   Result: ${cachedData ? 'HIT' : 'MISS'}`);
    if (cachedData) {
      console.log(`   Data:`, cachedData);
    }
    console.log('');

    // 7. Test placeholder resolution
    console.log('7. Placeholder resolution examples:');
    console.log(`   Template: ${cacheYaml.user._store.key}`);
    console.log(`   Resolved: ${userStoreKey}`);
    console.log('');

    // 8. Test tenant data load
    console.log('8. Loading tenant data...');
    const tenantStoreKey = resolvePlaceholders(
      cacheYaml.tenant._store.key,
      { tenant_id: testUser.tenant_id }
    );
    console.log(`   Key: ${tenantStoreKey}`);

    let tenantData = await kvsClient.get(tenantStoreKey);
    if (!tenantData) {
      const tenantRow = await dbClient.fetchOne(
        dbConfig,
        'tenants',
        `id=${testUser.tenant_id}`
      );
      if (tenantRow) {
        tenantData = {
          name: tenantRow.name,
          display_name: tenantRow.display_name
        };
        await kvsClient.set(tenantStoreKey, tenantData, cacheYaml.tenant._store.ttl);
      }
    }
    console.log(`   Tenant:`, tenantData);
    console.log('');

    console.log('=== All operations completed successfully ===');

  } catch (error) {
    console.error('Error:', error.message);
    throw error;
  } finally {
    // Cleanup
    await kvsClient.disconnect();
    await dbClient.closeAll();
  }
}

// Helper function to resolve placeholders
function resolvePlaceholders(template, params) {
  let result = template;
  for (const [key, value] of Object.entries(params)) {
    result = result.replace(new RegExp(`\\$\\{${key}\\}`, 'g'), String(value));
  }
  return result;
}

// Run
main().catch(console.error);
