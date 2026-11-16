// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! PlanPro 1.10's enumerations and the values of the signalling domain they
//! are read as (`mapping.js`, decision 0011): one table per attribute, read
//! both ways — a key as its value, and a value as the key the export writes
//! (the first key that reads as it).

/// A table: `$name` reads a key, `$back` gives the key a value is written as.
macro_rules! table {
    ($(#[$doc:meta])* $name:ident, $back:ident, { $($k:literal => $v:literal),* $(,)? }) => {
        $(#[$doc])*
        pub fn $name(key: &str) -> Option<&'static str> {
            match key {
                $($k => Some($v),)*
                _ => None,
            }
        }
        #[doc = concat!("The key a value of [`", stringify!($name), "`] is written as.")]
        pub fn $back(value: &str) -> Option<&'static str> {
            [$(($k, $v)),*].into_iter().find(|(_, v)| *v == value).map(|(k, _)| k)
        }
    };
}

table! {
    /// `SIGNAL_FUNCTION`.
    signal_function, signal_function_key, {
        "Einfahr_Signal" => "entry",
        "Ausfahr_Signal" => "exit",
        "Einfahr_Ausfahr_Signal" => "entryExit",
        "Zwischen_Signal" => "intermediate",
        "Ausfahr_Zwischen_Signal" => "exitIntermediate",
        "Einfahr_Block_Signal" => "entryBlock",
        "Block_Signal" => "block",
        "Gruppenausfahr_Signal" => "groupExit",
        "Gruppenzwischen_Signal" => "groupIntermediate",
        "Gruppenausfahr_Gruppenzwischen_Signal" => "groupExitIntermediate",
        "Nachrueck_Signal" => "followUp",
        "Deckungs_Signal" => "cover",
        "Zugdeckungs_Signal" => "trainCover",
        "Zug_Ziel_Signal" => "trainTarget",
        "BUE_Ueberwachungssignal" => "levelCrossing",
        "Vorsignal_Vorsignalwiederholer" => "distant",
        "Alleinstehendes_Zusatzsignal" => "standaloneIndicator",
        "andere" => "other",
    }
}

table! {
    /// `VIRTUAL_FUNCTION`.
    virtual_function, virtual_function_key, {
        "Zug_Start_ohne_Signal" => "trainStartWithoutSignal",
        "Zug_Ziel_ohne_Signal" => "trainTargetWithoutSignal",
        "Zug_Ziel_Strecke" => "trainTargetLine",
        "Zug_Start_Awanst" => "trainStartSiding",
        "Zug_Ziel_Awanst" => "trainTargetSiding",
        "Zug_Start_Mittelweiche" => "trainStartMiddleSwitch",
        "Zug_Start_Ziel_Bk" => "trainStartTargetBlock",
        "Zug_Start_Ziel_Bk_mit_Zs_1" => "trainStartTargetBlockZs1",
        "Zug_Start_Ziel_Bk_mit_Zs_7" => "trainStartTargetBlockZs7",
        "Rangier_Start_Ziel_ohne_Signal" => "shuntingStartTargetWithoutSignal",
        "Vsig_virtuell" => "virtualDistant",
        "sonstige" => "other",
    }
}

table! {
    /// `SIGNAL_TYPE`.
    signal_type, signal_type_key, {
        "Hauptsignal" => "main",
        "Hauptsperrsignal" => "mainShunting",
        "Mehrabschnittssignal" => "multiSection",
        "Mehrabschnittssperrsignal" => "multiSectionShunting",
        "Sperrsignal" => "shunting",
        "Vorsignal" => "distant",
        "Vorsignalwiederholer" => "distantRepeater",
        "Zugdeckungssignal" => "trainCover",
        "andere" => "other",
    }
}

table! {
    /// `SIGNAL_SYSTEM`.
    signal_system, signal_system_key, {
        "Ks" => "ks",
        "HV" => "hv",
        "SV" => "sv",
        "Hl" => "hl",
    }
}

table! {
    /// `MOUNTING`.
    mounting, mounting_key, {
        "Mast" => "mast",
        "Signalausleger" => "bracket",
        "Signalbruecke" => "gantry",
        "Fundament" => "foundation",
        "Sonderkonstruktion" => "special",
        "andere" => "other",
    }
}

table! {
    /// `FRAME`.
    frame, frame_key, {
        "Schirm" => "screen",
        "Zusatzanzeiger" => "indicator",
        "Blechtafel" => "plate",
        "Keramikkoerper" => "body",
        "Bezeichnungsschild" => "nameplate",
        "sonstige" => "other",
    }
}

table! {
    /// `DETECTION_POINT`.
    detection_point, detection_point_key, {
        "Isolierstoss_einschienig" => "insulatedJointOneRail",
        "Isolierstoss_zweischienig" => "insulatedJointTwoRail",
        "elektrischer_Trennstoss" => "electricalJoint",
    }
}

table! {
    /// `SECTION_KIND`.
    section_kind, section_kind_key, {
        "Achszaehlanlage" => "axleCounting",
        "TF_Gleisstromkreis" => "audioFrequencyCircuit",
        "NF_Gleisstromkreis" => "lowFrequencyCircuit",
        "FTGS" => "ftgs",
        "sonstige" => "other",
    }
}

table! {
    /// `OPERATION`.
    operation, operation_key, {
        "elektrisch_ferngestellt" => "electricRemote",
        "elektrisch_ortsgestellt" => "electricLocal",
        "mechanisch_ferngestellt" => "mechanicalRemote",
        "mechanisch_ortsgestellt" => "mechanicalLocal",
        "nicht_stellbar" => "notWorked",
        "Rueckfallweiche" => "springBack",
        "stillgelegt_links" => "disabledLeft",
        "stillgelegt_rechts" => "disabledRight",
        "sonstige" => "other",
    }
}

table! {
    /// `POSITION`.
    position, position_key, {
        "links" => "left",
        "rechts" => "right",
    }
}

table! {
    /// `VIA`.
    via, via_key, {
        "links" => "left",
        "rechts" => "right",
        "Spitze" => "tip",
    }
}

table! {
    /// `DERAILER_POSITION`.
    derailer_position, derailer_position_key, {
        "aufgelegt" => "applied",
        "abgelegt" => "removed",
    }
}

table! {
    /// `TWOFOLD_KIND`.
    twofold_kind, twofold_kind_key, {
        "echt" => "genuine",
        "echt_eigen" => "genuineOwn",
        "eigen" => "own",
    }
}

table! {
    /// `TWOFOLD_MEASURE`.
    twofold_measure, twofold_measure_key, {
        "Verschluss" => "locking",
        "Ersatzschutz" => "substitute",
    }
}

table! {
    /// `PZB_FREQUENCY`.
    pzb_frequency, pzb_frequency_key, {
        "500_Hz" => "500",
        "1000_Hz" => "1000",
        "2000_Hz" => "2000",
        "1000_2000_Hz" => "1000+2000",
    }
}

table! {
    /// `PZB_EFFECT`.
    pzb_effect, pzb_effect_key, {
        "staendig_wirksam" => "permanent",
        "schaltbar_von_Signal" => "switchedBySignal",
        "sonstige" => "other",
    }
}

table! {
    /// `PZB_ROUTE_EFFECT`.
    pzb_route_effect, pzb_route_effect_key, {
        "staendig_wirksam_wenn_Fahrstrasse_eingestellt" => "activeWhenSet",
        "unwirksam_wenn_Fahrstrasse_eingestellt" => "inactiveWhenSet",
        "sonstige" => "other",
    }
}

table! {
    /// `GUE_ARRANGEMENT`.
    gue_arrangement, gue_arrangement_key, {
        "1_EA" => "startFirst",
        "2_AE" => "endFirst",
    }
}

table! {
    /// `SIDE`.
    side, side_key, {
        "links" => "left",
        "rechts" => "right",
    }
}

table! {
    /// `SWITCH_KIND`.
    switch_kind, switch_kind_key, {
        "EW" => "switch",
        "ABW" => "switch",
        "IBW" => "switch",
        "ABW/IBW" => "switch",
        "IBW/ABW" => "switch",
        "DW" => "switch",
        "Eins. DW" => "switch",
        "Sym. ABW" => "switch",
        "Klothoidenweiche" => "switch",
        "Korbbogenweiche" => "switch",
    }
}

/// `GEO_FORM`: the element type of the core and, for a transition, its form.
pub fn geo_form(key: &str) -> Option<(u8, Option<&'static str>)> {
    Some(match key {
        "Gerade" => (0, None),
        "Bogen" => (1, None),
        "Klothoide" | "Uebergangsbogen_S_Form" => (2, Some("clothoid")),
        "Blosskurve" | "Bloss_einfach_geschwungen" | "S_Form_einfach_geschwungen" => (2, Some("bloss")),
        _ => return None,
    })
}

/// `COORDINATE_SYSTEM`: DB_REF in its Gauss-Krüger strips.
const COORDINATE_SYSTEMS: [(&str, i64); 4] = [("CR0", 5682), ("DR0", 5683), ("ER0", 5684), ("FR0", 5685)];

pub fn coordinate_system(key: &str) -> Option<i64> {
    COORDINATE_SYSTEMS.iter().find(|(k, _)| *k == key).map(|(_, c)| *c)
}

/// The key of a DB_REF strip's EPSG code.
pub fn coordinate_system_key(code: i64) -> Option<&'static str> {
    COORDINATE_SYSTEMS.iter().find(|(_, c)| *c == code).map(|(k, _)| *k)
}

/// `BUFFER_STOPS`: which end marks are buffer stops.
pub fn buffer_stop(kind: &str) -> bool {
    matches!(kind, "Bremsprellbock" | "Festprellbock" | "Schwellenkreuz" | "Kopframpe")
}
