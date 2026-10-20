use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default)]
pub struct Reloj {
    sincronizado: Arc<AtomicBool>,
}

impl Reloj {
    pub fn ahora(&self) -> Option<DateTime<Utc>> {
        self.sincronizado.load(Ordering::Relaxed).then(Utc::now)
    }

    /// Lanza la tarea de fondo que actualiza la bandera.
    /// Si `confiar_sistema` es true (desarrollo en Mac) no consulta nada.
    pub fn iniciar_vigilancia(&self, confiar_sistema: bool) {
        if confiar_sistema {
            tracing::warn!("CONFIAR_HORA_SISTEMA activo: no se verifica NTP");
            self.sincronizado.store(true, Ordering::Relaxed);
            return;
        }
        let bandera = self.sincronizado.clone();
        tokio::spawn(async move {
            loop {
                let ok = ntp_sincronizado().await;
                let antes = bandera.swap(ok, Ordering::Relaxed);
                if antes != ok {
                    // Se loguea solo el cambio, para no llenar el log cada 5s.
                    tracing::info!(sincronizado = ok, "cambio de estado de la hora");
                }
                let espera = if ok { 60 } else { 5 };
                tokio::time::sleep(Duration::from_secs(espera)).await;
            }
        });
    }
}

/// Máximo que se espera a `timedatectl`. Si dbus/systemd se traba, sin esto
/// el loop se quedaría esperando y la bandera quedaría congelada.
const LIMITE_TIMEDATECTL: Duration = Duration::from_secs(5);

/// Le pregunta a Linux si el reloj está sincronizado por NTP.
/// Cualquier fallo (comando no existe, salida rara, no responde a tiempo)
/// cuenta como "no".
async fn ntp_sincronizado() -> bool {
    let consulta = tokio::process::Command::new("timedatectl")
        .args(["show", "-p", "NTPSynchronized", "--value"])
        // Si el timeout abandona la espera, el proceso trabado se mata.
        .kill_on_drop(true)
        .output();

    match tokio::time::timeout(LIMITE_TIMEDATECTL, consulta).await {
        Ok(Ok(salida)) => String::from_utf8_lossy(&salida.stdout).trim() == "yes",
        Ok(Err(e)) => {
            tracing::debug!(error = %e, "no se pudo ejecutar timedatectl");
            false
        }
        Err(_) => {
            tracing::warn!("timedatectl no respondió a tiempo");
            false
        }
    }
}
