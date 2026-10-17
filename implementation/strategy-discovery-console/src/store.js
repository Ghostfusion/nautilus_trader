/**
 * The shared campaign store.
 *
 * One campaign object, recomputed deterministically from the options, is what the views read; only
 * the campaign view writes it, and the views downstream of a campaign - the acceptance record and
 * the audit - are rebuilt when it changes. Nothing here is a cache: `setOptions` runs the whole
 * pipeline again, because a campaign is cheap and a stale one would be a lie.
 */

import { DEFAULT_OPTIONS, runCampaign } from './engine/campaign.js';

const listeners = new Set();
let options = { ...DEFAULT_OPTIONS };
let campaign = runCampaign(options);

export const store = {
  /** The declared inputs of the current campaign. */
  getOptions() {
    return { ...options };
  },

  /** The current campaign's whole trace. */
  getCampaign() {
    return campaign;
  },

  /** Change one or more declared inputs and recompute the campaign for every subscriber. */
  setOptions(patch) {
    options = { ...options, ...patch };
    campaign = runCampaign(options);
    for (const listener of listeners) listener(campaign);
    return campaign;
  },

  /** Restore the defaults, which is what the check-out control does. */
  reset() {
    return store.setOptions({ ...DEFAULT_OPTIONS });
  },

  /** Run `listener` whenever the campaign changes. Returns the unsubscribe. */
  subscribe(listener) {
    listeners.add(listener);
    return () => listeners.delete(listener);
  },
};
