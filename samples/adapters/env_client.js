/**
 * ENVClient implementation
 *
 * Implements the ENVClient Required Port.
 * Provides access to environment variables.
 */

class ENVAdapter {
  /**
   * Get environment variable
   * @param {string} key - The environment variable name
   * @returns {string|null} The value or null if not found
   */
  get(key) {
    return process.env[key] ?? null;
  }

  /**
   * Check if environment variable exists
   * @param {string} key - The environment variable name
   * @returns {boolean} True if exists
   */
  has(key) {
    return key in process.env;
  }

  /**
   * Get all environment variables
   * @returns {Object} All environment variables
   */
  getAll() {
    return { ...process.env };
  }
}

module.exports = ENVAdapter;
