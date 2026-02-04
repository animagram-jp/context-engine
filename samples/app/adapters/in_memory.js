/**
 * InMemoryClient implementation
 *
 * Implements the InMemoryClient Required Port.
 * Manages in-memory key-value storage for the current process.
 */

class InMemoryAdapter {
  constructor() {
    this.data = new Map();
  }

  /**
   * Get value from in memory
   * @param {string} key - The key to retrieve
   * @returns {any|null} The value or null if not found
   */
  get(key) {
    return this.data.get(key) ?? null;
  }

  /**
   * Set value in in memory
   * @param {string} key - The key to set
   * @param {any} value - The value to store
   */
  set(key, value) {
    this.data.set(key, value);
  }

  /**
   * Delete value from in memory
   * @param {string} key - The key to delete
   * @returns {boolean} True if deleted, false if not found
   */
  delete(key) {
    return this.data.delete(key);
  }

  /**
   * Clear all data
   */
  clear() {
    this.data.clear();
  }

  /**
   * Get all keys
   * @returns {string[]} Array of all keys
   */
  keys() {
    return Array.from(this.data.keys());
  }
}

module.exports = InMemoryAdapter;
