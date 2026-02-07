/**
 * DBClient implementation using PostgreSQL
 *
 * Implements the DBClient Required Port.
 */

const { Pool } = require('pg');

class DBAdapter {
  constructor(config = null) {
    this.pools = new Map();

    if (config) {
      this.createPool('default', config);
    }
  }

  /**
   * Create a connection pool
   * @param {string} name - Pool name
   * @param {Object} config - Connection configuration (complete config object from State)
   *
   * Expected config structure:
   * {
   *   host: string,
   *   port: number,
   *   database: string,
   *   username: string,
   *   password: string,
   *   driver: string,      // e.g., 'mysql', 'postgres'
   *   charset: string,     // e.g., 'utf8mb4'
   *   collation: string    // e.g., 'utf8mb4_unicode_ci'
   * }
   */
  createPool(name, config) {
    const poolConfig = {
      host: config.host || process.env.DB_HOST,
      port: config.port || process.env.DB_PORT || 5432,
      database: config.database || process.env.DB_DATABASE,
      user: config.username || process.env.DB_USERNAME,
      password: config.password || process.env.DB_PASSWORD,
    };

    // Add charset/collation if provided (for MySQL/MariaDB compatibility)
    if (config.charset) {
      poolConfig.charset = config.charset;
    }

    const pool = new Pool(poolConfig);
    this.pools.set(name, pool);
  }

  /**
   * Get pool by name
   * @param {string} name - Pool name
   * @returns {Pool} PostgreSQL pool
   */
  getPool(name = 'default') {
    if (!this.pools.has(name)) {
      throw new Error(`Pool '${name}' not found`);
    }
    return this.pools.get(name);
  }

  /**
   * Fetch one record from database
   * @param {Object} config - Complete connection config object from State (not string!)
   * @param {string} table - Table name
   * @param {string|null} whereClause - WHERE clause (e.g., "id=123")
   * @returns {Promise<Object|null>} Record or null if not found
   *
   * NOTE: This implementation expects config to be an Object.
   * If you receive a string (connection name), you should maintain
   * your own connection map instead of calling State.
   */
  async fetchOne(config, table, whereClause = null) {
    const poolName = this.getPoolName(config);
    if (!this.pools.has(poolName)) {
      this.createPool(poolName, config);
    }

    const pool = this.getPool(poolName);

    let query = `SELECT * FROM ${table}`;
    if (whereClause) {
      query += ` WHERE ${whereClause}`;
    }
    query += ' LIMIT 1';

    const result = await pool.query(query);
    return result.rows.length > 0 ? result.rows[0] : null;
  }

  /**
   * Fetch all records from database
   * @param {Object} config - Connection config
   * @param {string} table - Table name
   * @param {string|null} whereClause - WHERE clause
   * @returns {Promise<Array>} Array of records
   */
  async fetchAll(config, table, whereClause = null) {
    const poolName = this.getPoolName(config);
    if (!this.pools.has(poolName)) {
      this.createPool(poolName, config);
    }

    const pool = this.getPool(poolName);

    let query = `SELECT * FROM ${table}`;
    if (whereClause) {
      query += ` WHERE ${whereClause}`;
    }

    const result = await pool.query(query);
    return result.rows;
  }

  /**
   * Execute a query
   * @param {Object} config - Connection config
   * @param {string} query - SQL query
   * @param {Array} params - Query parameters
   * @returns {Promise<number>} Number of affected rows
   */
  async execute(config, query, params = []) {
    const poolName = this.getPoolName(config);
    if (!this.pools.has(poolName)) {
      this.createPool(poolName, config);
    }

    const pool = this.getPool(poolName);
    const result = await pool.query(query, params);
    return result.rowCount;
  }

  /**
   * Generate pool name from config
   * @param {Object} config - Connection config
   * @returns {string} Pool name
   */
  getPoolName(config) {
    return `${config.host}_${config.database}`;
  }

  /**
   * Close all pools
   */
  async closeAll() {
    for (const [name, pool] of this.pools) {
      await pool.end();
      console.log(`Pool '${name}' closed`);
    }
    this.pools.clear();
  }
}

module.exports = DBAdapter;
