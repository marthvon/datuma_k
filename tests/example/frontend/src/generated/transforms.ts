
import type { Event, EventRecord, Venue, VenueRecord } from "./types";


function pad(n: number): string {
  if (n < 10) {
    return "0" + n;
  } else {
    return "" + n;
  }
}

export function toLocalDate(iso: string): Date {
  return new Date(iso);
}

export function toDatetimeLocalInput(value: Date): string {
  return value.getFullYear() + "-" + pad(value.getMonth() + 1) + "-" + pad(value.getDate()) + "T" + pad(value.getHours()) + ":" + pad(value.getMinutes());
}


export function parseEvent(raw: any): EventRecord {
  return {
    id: raw.id,
    title: raw.title,
    capacity: raw.capacity,
    starts_at: toLocalDate(raw.starts_at),

  };
}

export function serializeEvent(row: Event): Record<string, unknown> {
  return {
    title: row.title,
    capacity: row.capacity,
    starts_at: row.starts_at instanceof Date ? row.starts_at.toISOString() : toLocalDate("" + row.starts_at).toISOString(),

  };
}


export function parseVenue(raw: any): VenueRecord {
  return {
    id: raw.id,
    name: raw.name,
    capacity: raw.capacity,

  };
}

export function serializeVenue(row: Venue): Record<string, unknown> {
  return {
    name: row.name,
    capacity: row.capacity,

  };
}


