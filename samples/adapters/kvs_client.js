/**
 * KVSClient implementation using Redis
 *
 * Implements the KVSClient Required Port.
 */

const redis = require('redis');

class KVSAdapter {
  constructor(options = {}) {
    const host = options.host || process.env.REDIS_HOST || 'localhost';
    const port = options.port || process.env.REDIS_PORT || 6379;

    this.client = redis.createClient({
      socket: {
        host,
        port: parseInt(port)
      }
    });

    this.client.on('error', (err) => {
      console.error('Redis Client Error:', err);
    });

    this.connected = false;
  }

  /**
   * Connect to Redis
   */
  async connect() {
    if (!this.connected) {
      await this.client.connect();
      this.connected = true;
    }
  }

  /**
   * Disconnect from Redis
   */
  async disconnect() {
    if (this.connected) {
      await this.client.quit();
      this.connected = false;
    }
  }

  /**
   * Get value from KVS
   * @param {string} key - The key to retrieve
   * @returns {Promise<any|null>} The value or null if not found
   */
  async get(key) {
    await this.connect();
    const value = await this.client.get(key);
    if (value === null) return null;

    try {
      return JSON.parse(value);
    } catch (e) {
      return value;
    }
  }

  /**
   * Set value in KVS
   * @param {string} key - The key to set
   * @param {any} value - The value to store
   * @param {number|null} ttl - Time to live in seconds (optional)
   * @returns {Promise<boolean>} True if successful
   */
  async set(key, value, ttl = null) {
    await this.connect();
    const stringValue = typeof value === 'string' ? value : JSON.stringify(value);

    if (ttl !== null && ttl > 0) {
      await this.client.setEx(key, ttl, stringValue);
    } else {
      await this.client.set(key, stringValue);
    }

    return true;
  }

  /**
   * Delete value from KVS
   * @param {string} key - The key to delete
   * @returns {Promise<boolean>} True if deleted, false if not found
   */
  async delete(key) {
    await this.connect();
    const result = await this.client.del(key);
    return result > 0;
  }

  /**
   * Get TTL for a key
   * @param {string} key - The key to check
   * @returns {Promise<number>} TTL in seconds, -1 if no TTL, -2 if key doesn't exist
   */
  async ttl(key) {
    await this.connect();
    return await this.client.ttl(key);
  }
}

module.exports = KVSAdapter;
