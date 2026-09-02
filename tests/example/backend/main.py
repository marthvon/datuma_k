from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware

from generated.models import Event, EventRecord, Venue, VenueRecord

app = FastAPI()
app.add_middleware(
    CORSMiddleware,
    allow_origins=["http://localhost:5173"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


class Store:
    def __init__(self, record_cls):
        self.record_cls = record_cls
        self.rows = {}
        self.next_id = 1

    def list(self):
        return list(self.rows.values())

    def get(self, id):
        row = self.rows.get(id)
        if row is None:
            raise HTTPException(status_code=404, detail="not found")
        else:
            return row

    def create(self, payload):
        rec = self.record_cls(id=self.next_id, **payload.model_dump())
        self.rows[self.next_id] = rec
        self.next_id += 1
        return rec

    def update(self, id, payload):
        if id not in self.rows:
            raise HTTPException(status_code=404, detail="not found")
        else:
            rec = self.record_cls(id=id, **payload.model_dump())
            self.rows[id] = rec
            return rec

    def delete(self, id):
        if id not in self.rows:
            raise HTTPException(status_code=404, detail="not found")
        else:
            del self.rows[id]


events = Store(EventRecord)
venues = Store(VenueRecord)


@app.get("/events", response_model=list[EventRecord])
def list_events():
    return events.list()


@app.get("/events/{id}", response_model=EventRecord)
def get_event(id: int):
    return events.get(id)


@app.post("/events", response_model=EventRecord)
def create_event(payload: Event):
    return events.create(payload)


@app.put("/events/{id}", response_model=EventRecord)
def update_event(id: int, payload: Event):
    return events.update(id, payload)


@app.delete("/events/{id}", status_code=204)
def delete_event(id: int):
    events.delete(id)


@app.get("/venues", response_model=list[VenueRecord])
def list_venues():
    return venues.list()


@app.get("/venues/{id}", response_model=VenueRecord)
def get_venue(id: int):
    return venues.get(id)


@app.post("/venues", response_model=VenueRecord)
def create_venue(payload: Venue):
    return venues.create(payload)


@app.put("/venues/{id}", response_model=VenueRecord)
def update_venue(id: int, payload: Venue):
    return venues.update(id, payload)


@app.delete("/venues/{id}", status_code=204)
def delete_venue(id: int):
    venues.delete(id)
