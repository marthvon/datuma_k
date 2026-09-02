import { Link, Navigate, Route, Routes } from "react-router-dom";
import { EventDetail } from "./generated/EventDetail";
import { EventForm } from "./generated/EventForm";
import { EventList } from "./generated/EventList";
import { VenueDetail } from "./generated/VenueDetail";
import { VenueForm } from "./generated/VenueForm";
import { VenueList } from "./generated/VenueList";

function Home() {
  return (
    <section>
      <h1>Resources</h1>
      <ul>
        <li><Link to="/events">Events</Link></li>
        <li><Link to="/venues">Venues</Link></li>
      </ul>
    </section>
  );
}

export function App() {
  return (
    <>
      <nav>
        <Link to="/">Home</Link>
        <Link to="/events">Events</Link>
        <Link to="/venues">Venues</Link>
      </nav>
      <Routes>
        <Route path="/" element={<Home />} />
        <Route path="/events" element={<EventList />} />
        <Route path="/events/new" element={<EventForm />} />
        <Route path="/events/:id" element={<EventDetail />} />
        <Route path="/events/:id/edit" element={<EventForm />} />
        <Route path="/venues" element={<VenueList />} />
        <Route path="/venues/new" element={<VenueForm />} />
        <Route path="/venues/:id" element={<VenueDetail />} />
        <Route path="/venues/:id/edit" element={<VenueForm />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Routes>
    </>
  );
}
