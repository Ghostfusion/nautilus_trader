/**
 * Two kinds of held-back data: the roles, the reservation and the refusals.
 *
 * This view models `python/nautilus_trader/discovery/custody.py` and the page
 * `tutorials/strategy-discovery/two-kinds-of-held-back-data/README.md`. A reader names a lineage and
 * a data role and takes the campaign's one confirmation reservation, and is refused the second time
 * for the same lineage - the reservation is spent on the first request and nothing releases it. It
 * shows the reservation state as a table, the three roles, the per-lineage downgrade rule and the
 * closed refusal vocabulary.
 */

import {
  ADMISSIBLE_AS_VALIDATION,
  REFUSAL_CODES,
  ROLES,
  initialReservation,
  reserve,
} from '../../engine/holdout.js';
import { refusalCodes, reservationStates, transitions } from '../../data/holdout.js';

export const meta = {
  id: 'holdout',
  title: 'Two kinds of held-back data: the roles, the reservation and the refusals',
  order: 6,
  tutorial: 'tutorials/strategy-discovery/two-kinds-of-held-back-data/README.md',
};

export function mount(root, ctx) {
  const { ui, data } = ctx;
  const layer = data.layer(meta.id);

  const state = {
    reservation: initialReservation(),
    draftLineage: 'L-1',
    draftRole: 'final_confirmation',
    attempts: [],
  };

  root.appendChild(ui.block('The idea',
    ui.quote(layer.idea, layer.tutorial),
    ui.metrics(
      ui.metric('Grade', layer.grade),
      ui.metric('Tutorial', layer.tutorial),
    ),
  ));

  const modelHost = ui.block('The model');
  const refusalHost = ui.block('What it refuses');
  root.appendChild(modelHost);
  root.appendChild(refusalHost);

  root.appendChild(ui.block('Where it goes wrong', ui.list(layer.caveats)));
  root.appendChild(ui.block('Where this came from', ui.sources(layer.sources)));

  function render() {
    ui.clear(modelHost);

    modelHost.appendChild(ui.group('The three data roles',
      ui.table({
        columns: ['Role', 'Admissible as validation', 'Read by'],
        rows: ROLES().map((role) => [
          role.label,
          role.admissibleAsValidation ? 'yes' : 'no',
          role.readBy,
        ]),
        dense: true,
        caption: 'Only locked validation data may be read as validation evidence.',
      }),
    ));

    modelHost.appendChild(ui.field('Lineage', ui.textInput({
      id: 'holdout-lineage', value: state.draftLineage,
      onInput: (value) => { state.draftLineage = value; },
    }), 'The line of descent that would consume the holdout data; a reservation is bound to it.'));

    modelHost.appendChild(ui.field('Data role', ui.selectInput({
      id: 'holdout-role', value: state.draftRole,
      options: ROLES().map((role) => ({ value: role.name, label: role.label })),
      onChange: (value) => { state.draftRole = value; },
    }), 'The role the reservation is taken under; the final confirmation set is the rationed one.'));

    modelHost.appendChild(ui.button('Reserve holdout data', () => {
      const lineage = state.draftLineage || 'L-1';
      const result = reserve(state.reservation, lineage, state.draftRole);
      if (result.ok) {
        state.reservation = result.state;
        state.attempts = [{
          lineage, role: state.draftRole, answer: 'reserved', code: '',
        }, ...state.attempts];
      } else {
        state.attempts = [{
          lineage, role: state.draftRole, answer: 'refused', code: result.code,
        }, ...state.attempts];
      }
      render();
    }, { kind: 'primary' }));

    modelHost.appendChild(ui.group('The reservation state',
      ui.table({
        columns: ['Field', 'Value'],
        rows: [
          ['reserved', String(state.reservation.reserved)],
          ['lineage', state.reservation.lineage ?? 'none'],
          ['role', state.reservation.role ?? 'none'],
          ['consumed', String(state.reservation.consumed)],
        ],
        dense: true,
        caption: 'One campaign holds at most one reservation; it is spent on the first read and never released.',
      }),
    ));

    if (state.attempts.length > 0) {
      modelHost.appendChild(ui.group('Reservation attempts',
        ui.table({
          columns: ['Lineage', 'Role', 'Answer', 'Code'],
          rows: state.attempts.map((attempt) => [attempt.lineage, attempt.role, attempt.answer, attempt.code || '-']),
          dense: true,
        }),
      ));
    }

    modelHost.appendChild(ui.group('The reservation states',
      ui.table({
        columns: ['State', 'Label'],
        rows: reservationStates().map((state) => [state.name, state.label]),
        dense: true,
      }),
    ));

    modelHost.appendChild(ui.group('The state machine',
      ui.table({
        columns: ['#', 'From', 'Operation', 'Guard', 'To', 'Reservation'],
        rows: transitions().map((row) => [row.id, row.from, row.operation, row.guard, row.to, row.reservation]),
        dense: true,
        caption: 'The reservation is the one quantity that never moves back.',
      }),
    ));

    ui.clear(refusalHost);
    refusalHost.appendChild(ui.table({
      columns: ['Code', 'Condition', 'Message'],
      rows: refusalCodes().map((entry) => [
        entry.code,
        entry.condition,
        entry.message === null ? 'no raising path today' : entry.message,
      ]),
      dense: true,
    }));
    refusalHost.appendChild(ui.note(
      `A repeat for the same lineage is refused reservation_consumed: ${REFUSAL_CODES.length} custody codes are declared, and only locked validation (admissible: ${ADMISSIBLE_AS_VALIDATION('locked_validation')}) is admissible as validation.`,
      'warn',
    ));
  }

  // The sections were built as blocks; they become panels here so this view carries the same cards
  // as every other view. The section node itself is kept, because two of them are the hosts the
  // model renders into.
  const sections = [...root.children];
  ui.clear(root);
  for (const section of sections) {
    const heading = section.querySelector('h3');
    if (heading) heading.remove();
    root.appendChild(ui.panel({ title: heading ? heading.textContent : '' }, section));
  }

  render();
}
