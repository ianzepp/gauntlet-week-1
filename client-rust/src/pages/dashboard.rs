//! Dashboard page listing boards with create and open actions.
//!
//! Fetches the board list from `/api/boards` and renders a `BoardCard` for each.
//! Auth-guarded: redirects to `/login` if the user is not authenticated.
