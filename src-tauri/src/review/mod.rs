//! Fase 8 — Final review y papelera.
//!
//! Este módulo agrupa:
//! - `papelera`: borrado seguro vía la papelera del SO (renombrado
//!   desde `trash` para evitar colisión con la crate `trash`).
//! - `final_review`: la consulta que junta TODOS los `delete`
//!   acumulados de los 3 modos (Modo 1, 2, 3) para la pantalla
//!   de revisión final.
//!
//! La pantalla FinalReview es el "checkpoint" antes de ejecutar
//! el borrado: el usuario ve una lista única de todos los ficheros
//! marcados como `delete`, puede desmarcar los que no quiere (un
//! checkbox por fila), y al pulsar "Enviar a la papelera" se
//! ejecuta `trash::delete_all` con esos paths exactos.

pub mod final_review;
pub mod papelera;
