// SPDX-FileCopyrightText: 2026 Serigalan and the Rail-Across-Suite contributors
// SPDX-License-Identifier: LGPL-3.0-only

//! PlanPro 1.10's enumerations and the values of the signalling domain they
//! are read as (`mapping.js`, decision 0011): one table per attribute.

/// `SIGNAL_FUNCTION`.
pub fn signal_function(key: &str) -> Option<&'static str> {
    Some(match key {
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
        _ => return None,
    })
}

/// `VIRTUAL_FUNCTION`.
pub fn virtual_function(key: &str) -> Option<&'static str> {
    Some(match key {
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
        _ => return None,
    })
}

/// `SIGNAL_TYPE`.
pub fn signal_type(key: &str) -> Option<&'static str> {
    Some(match key {
        "Hauptsignal" => "main",
        "Hauptsperrsignal" => "mainShunting",
        "Mehrabschnittssignal" => "multiSection",
        "Mehrabschnittssperrsignal" => "multiSectionShunting",
        "Sperrsignal" => "shunting",
        "Vorsignal" => "distant",
        "Vorsignalwiederholer" => "distantRepeater",
        "Zugdeckungssignal" => "trainCover",
        "andere" => "other",
        _ => return None,
    })
}

/// `SIGNAL_SYSTEM`.
pub fn signal_system(key: &str) -> Option<&'static str> {
    Some(match key {
        "Ks" => "ks",
        "HV" => "hv",
        "SV" => "sv",
        "Hl" => "hl",
        _ => return None,
    })
}

/// `MOUNTING`.
pub fn mounting(key: &str) -> Option<&'static str> {
    Some(match key {
        "Mast" => "mast",
        "Signalausleger" => "bracket",
        "Signalbruecke" => "gantry",
        "Fundament" => "foundation",
        "Sonderkonstruktion" => "special",
        "andere" => "other",
        _ => return None,
    })
}

/// `FRAME`.
pub fn frame(key: &str) -> Option<&'static str> {
    Some(match key {
        "Schirm" => "screen",
        "Zusatzanzeiger" => "indicator",
        "Blechtafel" => "plate",
        "Keramikkoerper" => "body",
        "Bezeichnungsschild" => "nameplate",
        "sonstige" => "other",
        _ => return None,
    })
}

/// `DETECTION_POINT`.
pub fn detection_point(key: &str) -> Option<&'static str> {
    Some(match key {
        "Isolierstoss_einschienig" => "insulatedJointOneRail",
        "Isolierstoss_zweischienig" => "insulatedJointTwoRail",
        "elektrischer_Trennstoss" => "electricalJoint",
        _ => return None,
    })
}

/// `SECTION_KIND`.
pub fn section_kind(key: &str) -> Option<&'static str> {
    Some(match key {
        "Achszaehlanlage" => "axleCounting",
        "TF_Gleisstromkreis" => "audioFrequencyCircuit",
        "NF_Gleisstromkreis" => "lowFrequencyCircuit",
        "FTGS" => "ftgs",
        "sonstige" => "other",
        _ => return None,
    })
}

/// `OPERATION`.
pub fn operation(key: &str) -> Option<&'static str> {
    Some(match key {
        "elektrisch_ferngestellt" => "electricRemote",
        "elektrisch_ortsgestellt" => "electricLocal",
        "mechanisch_ferngestellt" => "mechanicalRemote",
        "mechanisch_ortsgestellt" => "mechanicalLocal",
        "nicht_stellbar" => "notWorked",
        "Rueckfallweiche" => "springBack",
        "stillgelegt_links" => "disabledLeft",
        "stillgelegt_rechts" => "disabledRight",
        "sonstige" => "other",
        _ => return None,
    })
}

/// `POSITION`.
pub fn position(key: &str) -> Option<&'static str> {
    Some(match key {
        "links" => "left",
        "rechts" => "right",
        _ => return None,
    })
}

/// `VIA`.
pub fn via(key: &str) -> Option<&'static str> {
    Some(match key {
        "links" => "left",
        "rechts" => "right",
        "Spitze" => "tip",
        _ => return None,
    })
}

/// `DERAILER_POSITION`.
pub fn derailer_position(key: &str) -> Option<&'static str> {
    Some(match key {
        "aufgelegt" => "applied",
        "abgelegt" => "removed",
        _ => return None,
    })
}

/// `TWOFOLD_KIND`.
pub fn twofold_kind(key: &str) -> Option<&'static str> {
    Some(match key {
        "echt" => "genuine",
        "echt_eigen" => "genuineOwn",
        "eigen" => "own",
        _ => return None,
    })
}

/// `TWOFOLD_MEASURE`.
pub fn twofold_measure(key: &str) -> Option<&'static str> {
    Some(match key {
        "Verschluss" => "locking",
        "Ersatzschutz" => "substitute",
        _ => return None,
    })
}

/// `PZB_FREQUENCY`.
pub fn pzb_frequency(key: &str) -> Option<&'static str> {
    Some(match key {
        "500_Hz" => "500",
        "1000_Hz" => "1000",
        "2000_Hz" => "2000",
        "1000_2000_Hz" => "1000+2000",
        _ => return None,
    })
}

/// `PZB_EFFECT`.
pub fn pzb_effect(key: &str) -> Option<&'static str> {
    Some(match key {
        "staendig_wirksam" => "permanent",
        "schaltbar_von_Signal" => "switchedBySignal",
        "sonstige" => "other",
        _ => return None,
    })
}

/// `PZB_ROUTE_EFFECT`.
pub fn pzb_route_effect(key: &str) -> Option<&'static str> {
    Some(match key {
        "staendig_wirksam_wenn_Fahrstrasse_eingestellt" => "activeWhenSet",
        "unwirksam_wenn_Fahrstrasse_eingestellt" => "inactiveWhenSet",
        "sonstige" => "other",
        _ => return None,
    })
}

/// `GUE_ARRANGEMENT`.
pub fn gue_arrangement(key: &str) -> Option<&'static str> {
    Some(match key {
        "1_EA" => "startFirst",
        "2_AE" => "endFirst",
        _ => return None,
    })
}

/// `SIDE`.
pub fn side(key: &str) -> Option<&'static str> {
    Some(match key {
        "links" => "left",
        "rechts" => "right",
        _ => return None,
    })
}

/// `SWITCH_KIND`.
pub fn switch_kind(key: &str) -> Option<&'static str> {
    Some(match key {
        "EW" => "turnout",
        "ABW" => "turnout",
        "IBW" => "turnout",
        "ABW/IBW" => "turnout",
        "IBW/ABW" => "turnout",
        "DW" => "turnout",
        "Eins. DW" => "turnout",
        "Sym. ABW" => "turnout",
        "Klothoidenweiche" => "turnout",
        "Korbbogenweiche" => "turnout",
        _ => return None,
    })
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
pub fn coordinate_system(key: &str) -> Option<i64> {
    Some(match key {
        "CR0" => 5682,
        "DR0" => 5683,
        "ER0" => 5684,
        "FR0" => 5685,
        _ => return None,
    })
}

/// `BUFFER_STOPS`: which end marks are buffer stops.
pub fn buffer_stop(kind: &str) -> bool {
    matches!(kind, "Bremsprellbock" | "Festprellbock" | "Schwellenkreuz" | "Kopframpe")
}
