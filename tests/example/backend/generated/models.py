
from datetime import datetime

from pydantic import BaseModel, Field



class Event(BaseModel):
    title: str
    capacity: int = Field(ge=1, le=500)
    starts_at: datetime


class EventRecord(Event):
    id: int


class Venue(BaseModel):
    name: str
    capacity: int = Field(ge=1, le=10000)


class VenueRecord(Venue):
    id: int


