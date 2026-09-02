
import { useEffect, useState } from "react";
import { Link, useNavigate, useParams } from "react-router-dom";
import { api } from "../api";
import type { EventRecord } from "./types";

export function EventDetail() {
  const params = useParams();
  const navigate = useNavigate();
  const [row, setRow] = useState(null as EventRecord | null);
  const [error, setError] = useState("");
  const id = Number(params.id);

  useEffect(() => {
    api.Event.get(id).then(setRow).catch(function (err) {
      setError(String(err));
    });
  }, [id]);

  if (error.length > 0) {
    return <p className="error">{error}</p>;
  } else if (row === null) {
    return <p>Loading...</p>;
  } else {
    return (
      <section>
        <p><Link to="/events">Back to list</Link></p>
        <h1>Event {row.id}</h1>
        <dl>
          <dt>title</dt>
          <dd>{row.title}</dd>
          <dt>capacity</dt>
          <dd>{row.capacity}</dd>
          <dt>starts_at</dt>
          <dd>{row.starts_at.toLocaleString()}</dd>

        </dl>
        <p>
          <Link to={"/events/" + row.id + "/edit"}>Edit</Link>
          {" "}
          <button type="button" onClick={function () {
            api.Event.remove(row.id).then(function () {
              navigate("/events");
            }).catch(function (err) {
              setError(String(err));
            });
          }}>Delete</button>
        </p>
      </section>
    );
  }
}
