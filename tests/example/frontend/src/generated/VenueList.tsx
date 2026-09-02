
import { useEffect, useState } from "react";
import { Link } from "react-router-dom";
import { api } from "../api";
import type { VenueRecord } from "./types";

export function VenueList() {
  const [rows, setRows] = useState([] as VenueRecord[]);
  const [error, setError] = useState("");

  useEffect(() => {
    api.Venue.list().then(setRows).catch(function (err) {
      setError(String(err));
    });
  }, []);

  if (error.length > 0) {
    return <p className="error">{error}</p>;
  } else {
    return (
      <section>
        <h1>Venues</h1>
        <p><Link to="/venues/new">Create</Link></p>
        <table>
          <thead>
            <tr>
              <th>id</th>
              <th>name</th>
              <th>capacity</th>

              <th></th>
            </tr>
          </thead>
          <tbody>
            {rows.map(function (row) {
              return (
                <tr key={row.id}>
                  <td>{row.id}</td>
                  <td>{row.name}</td>
                  <td>{row.capacity}</td>

                  <td>
                    <Link to={"/venues/" + row.id}>View</Link>
                    {" "}
                    <Link to={"/venues/" + row.id + "/edit"}>Edit</Link>
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
