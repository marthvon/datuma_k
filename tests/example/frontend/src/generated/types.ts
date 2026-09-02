

export type Event = {
  title: string;
  capacity: number;
  starts_at: Date;

};

export type EventRecord = { id: number } & Event;


export type Venue = {
  name: string;
  capacity: number;

};

export type VenueRecord = { id: number } & Venue;


