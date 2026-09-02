import { parseEvent, parseVenue, serializeEvent, serializeVenue } from "./generated/transforms";
import type { Event, EventRecord, Venue, VenueRecord } from "./generated/types";

const BASE = "http://localhost:8000";

async function request(method: string, path: string, body?: unknown): Promise<any> {
  const init: RequestInit = { method, headers: { "Content-Type": "application/json" } };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
  }
  const res = await fetch(BASE + path, init);
  if (!res.ok) {
    throw new Error(method + " " + path + " failed: " + res.status);
  } else if (res.status === 204) {
    return undefined;
  } else {
    return res.json();
  }
}

export const api = {
  Event: {
    list: async (): Promise<EventRecord[]> => (await request("GET", "/events")).map(parseEvent),
    get: async (id: number): Promise<EventRecord> => parseEvent(await request("GET", "/events/" + id)),
    create: async (row: Event): Promise<EventRecord> => parseEvent(await request("POST", "/events", serializeEvent(row))),
    update: async (id: number, row: Event): Promise<EventRecord> => parseEvent(await request("PUT", "/events/" + id, serializeEvent(row))),
    remove: async (id: number): Promise<void> => {
      await request("DELETE", "/events/" + id);
    },
  },
  Venue: {
    list: async (): Promise<VenueRecord[]> => (await request("GET", "/venues")).map(parseVenue),
    get: async (id: number): Promise<VenueRecord> => parseVenue(await request("GET", "/venues/" + id)),
    create: async (row: Venue): Promise<VenueRecord> => parseVenue(await request("POST", "/venues", serializeVenue(row))),
    update: async (id: number, row: Venue): Promise<VenueRecord> => parseVenue(await request("PUT", "/venues/" + id, serializeVenue(row))),
    remove: async (id: number): Promise<void> => {
      await request("DELETE", "/venues/" + id);
    },
  },
};
