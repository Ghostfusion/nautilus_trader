import { describe, expect, test } from 'bun:test';

import {
  ADMISSIBLE_AS_VALIDATION,
  REFUSAL_CODES,
  ROLES,
  downgrade,
  initialReservation,
  reserve,
} from '../src/engine/holdout.js';
import { refusalCodes, roles, transitions } from '../src/data/holdout.js';

describe('the data roles', () => {
  test('exactly three roles are declared, and only locked validation is admissible', () => {
    // Breaks the classification if another role is silently treated as validation evidence.
    expect(ROLES().map((role) => role.name)).toEqual(['development', 'locked_validation', 'final_confirmation']);
    expect(ROLES().filter((role) => role.admissibleAsValidation).map((role) => role.name)).toEqual(['locked_validation']);
    expect(ADMISSIBLE_AS_VALIDATION('locked_validation')).toBe(true);
    expect(ADMISSIBLE_AS_VALIDATION('development')).toBe(false);
  });

  test('the roles and their readers match the data module', () => {
    expect(ROLES()).toEqual(roles());
  });
});

describe('the confirmation reservation', () => {
  test('a fresh campaign holds no reservation', () => {
    expect(initialReservation()).toEqual({ reserved: false, lineage: null, role: null, consumed: false });
  });

  test('a first reservation is taken for the named lineage and role', () => {
    const result = reserve(initialReservation(), 'L-1', 'final_confirmation');
    expect(result.ok).toBe(true);
    expect(result.state.lineage).toBe('L-1');
    expect(result.state.reserved).toBe(true);
  });

  test('a second reservation for the same lineage is refused reservation_consumed', () => {
    // Breaks the page's central claim if a campaign could reserve the same lineage twice.
    const first = reserve(initialReservation(), 'L-1', 'final_confirmation');
    const second = reserve(first.state, 'L-1', 'final_confirmation');
    expect(second.ok).toBe(false);
    expect(second.code).toBe('reservation_consumed');
    expect(second.reason).toContain('nothing releases it');
  });

  test('a reservation for a different lineage is refused second_candidate', () => {
    const first = reserve(initialReservation(), 'L-1', 'final_confirmation');
    const other = reserve(first.state, 'L-2', 'locked_validation');
    expect(other.ok).toBe(false);
    expect(other.code).toBe('second_candidate');
  });

  test('a role outside the declared three is refused', () => {
    const result = reserve(initialReservation(), 'L-1', 'development-ish');
    expect(result.ok).toBe(false);
    expect(result.code).toBe('role_mismatch');
  });

  test('a refusal leaves the reservation where it was', () => {
    const first = reserve(initialReservation(), 'L-1', 'final_confirmation');
    reserve(first.state, 'L-1', 'final_confirmation');
    expect(first.state.lineage).toBe('L-1');
    expect(first.state.consumed).toBe(false);
  });
});

describe('the per-lineage downgrade', () => {
  test('a partition a lineage consumed becomes development for it', () => {
    // Breaks the per-lineage rule (criterion 17) if a consumed partition stays validation.
    const result = downgrade([
      { partition: 'P1', lineage: 'L-1', role: 'locked_validation', by: 'C-1' },
      { partition: 'P2', lineage: 'L-2', role: 'locked_validation', by: 'C-1' },
    ], 'L-1');
    expect(result.ok).toBe(true);
    expect(result.downgraded).toEqual(['P1']);
    expect(result.lineages[0].role).toBe('development');
    expect(result.lineages[0].reason).toBe('C-1 was evaluated on P1 and its result informed the search');
    expect(result.lineages[1].role).toBe('locked_validation');
  });

  test('a lineage that consumed nothing has nothing to downgrade and is refused role_mismatch', () => {
    const result = downgrade([{ partition: 'P1', lineage: 'L-2', role: 'locked_validation', by: 'C-1' }], 'L-1');
    expect(result.ok).toBe(false);
    expect(result.code).toBe('role_mismatch');
    expect(result.reason).toBe('the lineage has never consumed a result on this partition');
  });
});

describe('the transcription and the model agree', () => {
  test('the custody refusal vocabulary is the twelve declared codes', () => {
    expect(REFUSAL_CODES.length).toBe(12);
    expect(REFUSAL_CODES).toContain('not_frozen');
    expect(REFUSAL_CODES).toContain('reservation_consumed');
    expect(refusalCodes().map((entry) => entry.code)).toEqual(REFUSAL_CODES);
  });

  test('two codes are declared with no raising path and are shown as such', () => {
    const withoutMessage = refusalCodes().filter((entry) => entry.message === null).map((entry) => entry.code);
    expect(withoutMessage).toEqual(['exploratory_label_required', 'confirmation_access_without_reservation']);
  });

  test('the transition table never releases the reservation', () => {
    // Breaks the "spent once and never released" claim if any transition shows it moving back.
    expect(transitions().length).toBe(10);
    expect(transitions().every((row) => ['taken', 'spent', 'unchanged (spent)', 'unchanged'].includes(row.reservation))).toBe(true);
  });
});
