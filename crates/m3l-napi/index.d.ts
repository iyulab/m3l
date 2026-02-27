/* eslint-disable */

/**
 * Parse a single M3L file and return the AST as JSON.
 *
 * @param content - M3L markdown text
 * @param filename - Source filename for error reporting
 * @returns JSON string with `{ success: boolean, data?: AST, error?: string }`
 */
export function parse(content: string, filename: string): string;

/**
 * Parse multiple M3L files and return the merged AST as JSON.
 *
 * @param filesJson - JSON array of `{ content: string, filename: string }` objects
 * @returns JSON string with `{ success: boolean, data?: AST, error?: string }`
 */
export function parseMulti(filesJson: string): string;

/**
 * Validate M3L content and return diagnostics as JSON.
 *
 * @param content - M3L markdown text
 * @param optionsJson - JSON options `{ strict?: boolean, filename?: string }`
 * @returns JSON string with `{ success: boolean, data?: ValidateResult, error?: string }`
 */
export function validate(content: string, optionsJson: string): string;
