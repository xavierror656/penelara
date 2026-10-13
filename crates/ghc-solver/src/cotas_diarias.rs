//! Cotas diarias que construye MotHor para cada una de sus dos tablas de carga.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AjusteCalculado {
    Automatico,
    UnaHoraMenos,
    UnaHoraMas,
}

impl AjusteCalculado {
    pub fn desde_xml(valor: &str) -> Option<Self> {
        match valor {
            "automatico" => Some(Self::Automatico),
            "unaHoraMenos" => Some(Self::UnaHoraMenos),
            "unaHoraMas" => Some(Self::UnaHoraMas),
            _ => None,
        }
    }

    fn unidades(self) -> i64 {
        match self {
            Self::Automatico => 0,
            Self::UnaHoraMenos => -1,
            Self::UnaHoraMas => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaCotasDiarias {
    /// Carga semanal de la tabla elegida; MotHor usa dos acumulados separados.
    pub carga_semanal: i32,
    /// Días efectivos ya calculados para este profesor y esta tabla.
    pub dias_efectivos: i32,
    /// Unidades de carga por período (`contexto+0x46d8`).
    pub unidades_por_periodo: i32,
    /// Rama `contexto+8 != 0`: redondea la carga dividida por la escala.
    pub redondear_carga: bool,
    /// Suelo del máximo en `registro+0x72`.
    pub suelo_maximo: i32,
    pub ajuste_maximo: AjusteCalculado,
    pub ajuste_minimo: AjusteCalculado,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CotasDiarias {
    pub minimo: i32,
    pub maximo: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaDiasEfectivos {
    /// Número de días de la cuadrícula usada por la tabla elegida.
    pub dias_calendario: u32,
    /// Días descartados al recorrer las marcas diarias de esa tabla.
    pub dias_descartados: u32,
    /// `periodosLibres/cantidadDeDias`, copiado en `registro+0x5c`.
    pub dias_libres_solicitados: u32,
    /// La rama `registro+0x5e == 0` compara solicitud y descarte.
    pub modo_5e_cero: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiasEfectivos {
    /// Valor guardado en `registro+0x80` o `+0x82`.
    pub en_registro: i32,
    /// Divisor de la fórmula de límites; el binario sustituye cero por uno.
    pub para_division: i32,
}

/// Reconstruye el descuento común de días tras contar los descartes de una tabla.
/// El conteo de `dias_descartados` depende de marcas nativas aún no portadas.
pub fn calcular_dias_efectivos(entrada: EntradaDiasEfectivos) -> Option<DiasEfectivos> {
    if entrada.dias_descartados > entrada.dias_calendario {
        return None;
    }
    let descuento = if entrada.modo_5e_cero {
        entrada
            .dias_descartados
            .max(entrada.dias_libres_solicitados)
    } else {
        entrada.dias_descartados
    };
    // Un descuento mayor que el calendario entra en una rama no contrastada.
    if descuento > entrada.dias_calendario {
        return None;
    }
    let disponibles = entrada.dias_calendario - descuento;
    let en_registro = i32::try_from(disponibles).ok()?;
    Some(DiasEfectivos {
        en_registro,
        para_division: en_registro.max(1),
    })
}

/// Reconstruye la rama observada para cargas no negativas y ajustes 0/1/2.
/// Devuelve `None` si los parámetros no son válidos o el resultado no cabe en i32.
pub fn calcular_cotas_diarias(entrada: EntradaCotasDiarias) -> Option<CotasDiarias> {
    let carga = i64::from(entrada.carga_semanal);
    let escala = i64::from(entrada.unidades_por_periodo);
    if carga < 0 || escala <= 0 || entrada.dias_efectivos < 0 {
        return None;
    }
    let dias = i64::from(entrada.dias_efectivos.max(1));
    let carga_escalada = if entrada.redondear_carga {
        // Para cargas positivas, esto reproduce `round(carga / escala)` del CRT.
        (carga + escala / 2) / escala
    } else {
        carga
    };
    let base = ((carga_escalada - 1) / dias + 2) * escala;
    let maximo =
        (base + entrada.ajuste_maximo.unidades() * escala).max(i64::from(entrada.suelo_maximo));
    let minimo = (base - 3 * escala + entrada.ajuste_minimo.unidades() * escala).max(0);
    Some(CotasDiarias {
        minimo: i32::try_from(minimo).ok()?,
        maximo: i32::try_from(maximo).ok()?,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntradaPuntuacionDiaria {
    /// Marca que permite puntuar esta tabla en el día evaluado.
    pub dia_habilitado: bool,
    pub carga_diaria: i32,
    pub cotas: CotasDiarias,
    /// `registro+0x5e == 0` omite las celdas vacías cuando hay días libres pedidos.
    pub modo_5e_cero: bool,
    pub dias_libres_solicitados: u32,
    /// `ponderable` escribe el modo nativo 1; los demás modos toman la rama 101.
    pub maximo_ponderable: bool,
    pub minimo_ponderable: bool,
    /// Criterio nativo `[contexto+0xbc]+0x0b`.
    pub peso: u8,
}

/// Contribución de una tabla y un día en las dos pasadas de `0x459fc0`.
/// La función nativa añade además otros términos antes y después de estas pasadas.
pub fn puntuacion_carga_en_dia(entrada: EntradaPuntuacionDiaria) -> i64 {
    if !entrada.dia_habilitado
        || (entrada.modo_5e_cero
            && entrada.dias_libres_solicitados > 0
            && entrada.carga_diaria == 0)
    {
        return 0;
    }
    let carga = i64::from(entrada.carga_diaria);
    let minimo = i64::from(entrada.cotas.minimo);
    let maximo = i64::from(entrada.cotas.maximo);
    let peso = i64::from(entrada.peso);
    if carga == minimo || carga == maximo {
        peso
    } else if carga > maximo {
        if entrada.maximo_ponderable {
            (carga - maximo + 1) * peso
        } else {
            101
        }
    } else if carga < minimo {
        if entrada.minimo_ponderable {
            (minimo - carga + 1) * peso
        } else {
            101
        }
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        calcular_cotas_diarias, calcular_dias_efectivos, puntuacion_carga_en_dia,
        AjusteCalculado as A, CotasDiarias, DiasEfectivos, EntradaCotasDiarias,
        EntradaDiasEfectivos, EntradaPuntuacionDiaria,
    };

    #[test]
    fn dias_efectivos_en_capturas_y_ramas_nativas() {
        let mut entrada = EntradaDiasEfectivos {
            dias_calendario: 5,
            dias_descartados: 0,
            dias_libres_solicitados: 0,
            modo_5e_cero: true,
        };
        assert_eq!(
            calcular_dias_efectivos(entrada),
            Some(DiasEfectivos {
                en_registro: 5,
                para_division: 5
            })
        );
        entrada.dias_libres_solicitados = 1;
        assert_eq!(calcular_dias_efectivos(entrada).unwrap().en_registro, 4);
        entrada.dias_libres_solicitados = 2;
        assert_eq!(calcular_dias_efectivos(entrada).unwrap().en_registro, 3);
        entrada.dias_descartados = 3;
        assert_eq!(calcular_dias_efectivos(entrada).unwrap().en_registro, 2);
        entrada.modo_5e_cero = false;
        entrada.dias_libres_solicitados = 5;
        assert_eq!(calcular_dias_efectivos(entrada).unwrap().en_registro, 2);
        entrada.dias_descartados = 0;
        entrada.modo_5e_cero = true;
        assert_eq!(
            calcular_dias_efectivos(entrada),
            Some(DiasEfectivos {
                en_registro: 0,
                para_division: 1
            })
        );
    }

    fn entrada(carga: i32, dias: i32, escala: i32, redondear: bool) -> EntradaCotasDiarias {
        EntradaCotasDiarias {
            carga_semanal: carga,
            dias_efectivos: dias,
            unidades_por_periodo: escala,
            redondear_carga: redondear,
            suelo_maximo: escala,
            ajuste_maximo: A::Automatico,
            ajuste_minimo: A::Automatico,
        }
    }

    #[test]
    fn reproduce_registros_nativos_de_carga_y_dias_libres() {
        let casos = [
            (0, 0, 1, false, 0, 1),  // Raquel sin sesiones y reunión no lectiva
            (5, 5, 1, false, 0, 2),  // Raquel, entrada base
            (6, 5, 1, false, 0, 3),  // reunión lectiva de Raquel
            (17, 5, 1, false, 2, 5), // Alicia, entrada base
            (9, 4, 1, false, 1, 4),  // Alfonso, un día libre
            (9, 3, 1, false, 1, 4),  // Alfonso, dos días libres
            (36, 5, 4, true, 0, 12), // Alicia, duración T
            (36, 4, 4, true, 4, 16), // Alicia, duración T y un día libre
            (20, 5, 4, true, 0, 8),  // Raquel, duración T
            (24, 5, 4, true, 0, 12), // Raquel, reunión lectiva en duración T
        ];
        for (carga, dias, escala, redondear, minimo, maximo) in casos {
            assert_eq!(
                calcular_cotas_diarias(entrada(carga, dias, escala, redondear)),
                Some(CotasDiarias { minimo, maximo })
            );
        }
    }

    #[test]
    fn aplica_ajustes_independientes_de_maximo_y_minimo() {
        let mut caso = entrada(9, 5, 1, false);
        caso.ajuste_maximo = A::UnaHoraMas;
        caso.ajuste_minimo = A::UnaHoraMas;
        assert_eq!(
            calcular_cotas_diarias(caso),
            Some(CotasDiarias {
                minimo: 1,
                maximo: 4
            })
        );
        caso.ajuste_maximo = A::UnaHoraMenos;
        caso.ajuste_minimo = A::UnaHoraMenos;
        assert_eq!(
            calcular_cotas_diarias(caso),
            Some(CotasDiarias {
                minimo: 0,
                maximo: 2
            })
        );
        assert_eq!(A::desde_xml("unaHoraMas"), Some(A::UnaHoraMas));
        assert_eq!(A::desde_xml("ponderable"), None);
    }

    #[test]
    fn puntua_limites_igualdad_exceso_y_dias_libres() {
        let mut caso = EntradaPuntuacionDiaria {
            dia_habilitado: true,
            carga_diaria: 3,
            cotas: CotasDiarias {
                minimo: 1,
                maximo: 4,
            },
            modo_5e_cero: true,
            dias_libres_solicitados: 0,
            maximo_ponderable: true,
            minimo_ponderable: true,
            peso: 3,
        };
        assert_eq!(puntuacion_carga_en_dia(caso), 0);
        caso.carga_diaria = 4;
        assert_eq!(puntuacion_carga_en_dia(caso), 3);
        caso.carga_diaria = 1;
        assert_eq!(puntuacion_carga_en_dia(caso), 3);
        caso.carga_diaria = 5;
        assert_eq!(puntuacion_carga_en_dia(caso), 6);
        caso.maximo_ponderable = false;
        assert_eq!(puntuacion_carga_en_dia(caso), 101);
        caso.carga_diaria = 0;
        assert_eq!(puntuacion_carga_en_dia(caso), 6);
        caso.dias_libres_solicitados = 1;
        assert_eq!(puntuacion_carga_en_dia(caso), 0);
        caso.modo_5e_cero = false;
        assert_eq!(puntuacion_carga_en_dia(caso), 6);
        caso.minimo_ponderable = false;
        assert_eq!(puntuacion_carga_en_dia(caso), 101);
        caso.dia_habilitado = false;
        assert_eq!(puntuacion_carga_en_dia(caso), 0);
    }
}
