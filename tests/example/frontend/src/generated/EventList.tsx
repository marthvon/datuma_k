
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../api";
import type { EventRecord } from "./types";

export function EventList() {
  const [rows, setRows] = useState([] as EventRecord[]);
  const [error, setError] = useState("");

  useEffect(() => {
    api.Event.list().then(setRows).catch(function (err) {
      setError(String(err));
    });
  }, []);

  if (error.length > 0) {
    return <p className="error">{error}</p>;
  } else {
    return (
      <section>
        <h1>Events</h1>
        <p><Link to="/events/new">Create</Link></p>
        <table>
          <thead>
            <tr>
              <th>id</th>
              <th>title</th>
              <th>capacity</th>
              <th>starts_at</th>

              <th></th>
            </tr>
          </thead>
          <tbody>
            {rows.map(function (row) {
              return (
                <tr key={row.id}>
                  <td>{row.id}</td>
                  <td>{row.title}</td>
                  <td>{row.capacity}</td>
                  <td>{row.starts_at.toLocaleString()}</td>

                  <td>
                    <Link to={"/events/" + row.id}>View</Link>
                    {" "}
                    <Link to={"/events/" + row.id + "/edit"}>Edit</Link>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </section>
    );
  }
}
