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

/**
 * Validate a multi-file M3L model and return diagnostics as JSON.
 *
 * The validation counterpart of `parseMulti`: cross-file constructs (inheritance, interface
 * references, a custom `::attribute` registered in another file) only resolve when the whole
 * file set is validated as one unit. Each diagnostic carries its own `file`.
 *
 * @param filesJson - JSON array of `{ content: string, filename: string }` objects
 * @param optionsJson - JSON options `{ strict?: boolean }`
 * @returns JSON string with `{ success: boolean, data?: ValidateResult, error?: string }`
 */
export function validateMulti(filesJson: string, optionsJson: string): string;

/**
 * Lint M3L content and return diagnostics as JSON.
 *
 * @param content - M3L markdown text
 * @param configJson - JSON config `{ rules?: Record<string, "off"|"warn"|"error"> }`
 * @returns JSON string with `{ success: boolean, data?: LintResult, error?: string }`
 */
export function lint(content: string, configJson: string): string;
