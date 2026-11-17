# Zweck

Dieses Dokument nennt die **Annahmen der Simulation** der Rail-Across-Suite: was sie aus einem Projekt in
rail-across-1.0 liest, wie sie daraus Fahrten, Belegungen, Kapazitäten und Bauszenarien rechnet und was sie dabei
vereinfacht. Es ergänzt die Formatbeschreibung rail-across-1.0 (Fachbereich timetables 1.2). Diese sagt, was
gespeichert wird; dieses Dokument sagt, was die Werkzeuge damit rechnen.

Gerechnet wird in vier Werkzeugen:

- im **Fahrplan-Editor** und im **Simulation-Tool**: die Fahrt jedes Zuges, seine Zeiten und seine Belegung und wie
  die Züge miteinander auskommen;
- in der **Kapazitäts-Sim**: die Kapazität von Streckenabschnitten und Bahnhöfen nach UIC 406, mit den
  Konfliktbereichen und den ausschlaggebenden Belegungselementen in einem schematischen Lageplan;
- in der **Baubetriebs-Sim**: welche Gleise und Weichen an einem Bauzustand in Betrieb sind und wie ein Fahrplan in
  einem Bauszenario fährt, mit gesperrten Gleisstücken, herausgenommenen und umgeleiteten Zügen.

Die Simulation folgt sechs Grundsätzen:

- **Die Physik von OSRD.** Die Fahrzeitrechnung ist die Hüllkurvensimulation von OSRD, dem Open Source Railway
  Designer der OpenRail Association, nach Rust portiert (`sim/`, Stand OSRD `fda69ee1`). Was OSRD rechnet, rechnet
  die Suite gleich, mit einer Ausnahme: Gebremst wird abhängig von der Neigung. Was nicht portiert ist, steht in
  Abschnitt „Grenzen“.
- **Die Infrastruktur, wie sie entworfen ist.** Neigungen, Bögen, Geschwindigkeiten, Halteplätze und Bahnsteige
  kommen aus den Entwurfswerten von Layout-Tool und Signalling-Tool, nicht aus gezeichneten Linien. Wo Daten fehlen,
  rechnet die Simulation mit einem neutralen Wert (eben, ohne Grenze) und **meldet die Lücke**, statt etwas zu
  erfinden. Nur wo ein Zug gar nicht halten kann, wird er nicht gerechnet.
- **Das Netz eines Tages.** Es gibt, was die Lebensdauern aus dem Layout-Tool an diesem Tag in Betrieb setzen. Ein
  Bauszenario sperrt darüber hinaus Gleisstücke eine Zeit lang.
- **Erst jeder Zug für sich, dann alle zusammen.** Jeder Zug wird zuerst allein gerechnet. Seine Fahrt ergibt
  seine Sperrzeiten; erst die Disposition bringt die Züge zusammen und rechnet einen Zug neu, wo er warten oder
  umgeleitet werden muss.
- **Bestimmt, nicht zufällig.** Dieselben Eingaben geben dieselben Ergebnisse. Es gibt keine Zufallsgrößen und keine
  Streuung.
- **SI-Einheiten.** Meter, Sekunden, m/s, m/s², kg, N; Neigungen in ‰ (m/km), Steigung positiv in Fahrtrichtung.
  Uhrzeiten sind Sekunden ab Mitternacht des Fahrplantags. Geschwindigkeiten des Formats in km/h werden beim
  Einlesen in m/s umgerechnet.

# Vom Fahrplan zur Belegung

Ein Zug durchläuft zehn Schritte. Die Abschnitte dieses Dokuments folgen ihnen.

1. **Orte und Halte:** Die Punkte des Zuges werden Bahnhöfen, Haltepunkten und Gleisenden des Netzes zugeordnet und
   jeder Halt einem Halteplatz, den der Zug nutzen darf.
2. **Weg:** der Weg über das Netz durch alle Punkte, mit den verlangten Richtungswechseln.
3. **Streckeneigenschaften:** Länge, Neigungen (mit den Bögen) und zulässige Geschwindigkeiten entlang des Wegs.
4. **Zug:** Fahrzeug und Traktionsart.
5. **Fahrzeitrechnung:** die schnellste Fahrt, die Fahrt mit Zuschlag und die Fahrt, die der Fahrplan hält,
   gebremst auf der Neigung.
6. **Uhr:** die Fahrt in Uhrzeit, mit den Abfahrtszeiten, auf die der Zug wartet.
7. **Blöcke und Sperrzeiten:** was der Zug wann belegt.
8. **Disposition:** Konflikte zeigen oder die Züge nacheinander ins Netz geben.
9. **Kapazität:** die Sperrzeiten verdichtet nach UIC 406, mit dem, was die Züge bindet und wo sie sich begegnen.
10. **Bauszenario:** der Fahrplan noch einmal an einem Tag mit gesperrten Stücken, die betroffenen Züge umgeleitet.

# Orte: Bahnhöfe, Haltepunkte und Gleisenden

Ein Punkt eines Zuges nennt eine Betriebsstelle (`operationalPointId`) oder ein Gleisende (`endMarkId`) über ihre
Kennung. Die Simulation macht daraus Orte:

<!-- cols: 22%, 1fr -->
| Ort | Was dazugehört |
|---|---|
| Bahnhof | die Bahnhofsgleise (`trackType` `station`) einer Betriebsstelle mit den Bahnsteigen daran; hat eine Betriebsstelle keine Bahnhofsgleise, aber Halteplätze, dann die Gleise ihrer Halteplätze und Bahnsteige, jeweils nur so weit, wie diese reichen |
| Haltepunkt | eine Betriebsstelle nur mit Bahnsteigen an den Streckengleisen, ohne eigene Gleise |
| Gleisende | ein Prellbock oder eine Betrachtungsgrenze, ein Ort am Ende eines Gleises |

- Ein Zug **beginnt und endet** nur in einem Bahnhof oder an einem Gleisende. Ein Haltepunkt ist kein Anfang und
  kein Ende, ein Gleisende liegt nie unterwegs.
- **Wo er nur durchfährt,** kommt jedes Gleis des Bahnhofs in Frage. Eine Durchfahrt wird in der Mitte dessen
  gezeitet, was der Bahnhof entlang des Wegs abdeckt.
- **An einer Betrachtungsgrenze** fährt ein Zug mit der Geschwindigkeit der Grenze ein und höchstens mit ihr aus. Hat
  die Grenze keine Geschwindigkeit, gilt, was Strecke und Zug dort zulassen. An einem Prellbock steht er.

# Wo ein Zug hält

Ein Zug hält nur an einem **Halteplatz** (`stoppingPlaces` aus dem Signalling-Tool), auf dem verlangten Gleis, wo ein
Punkt eines verlangt. Das gilt für seinen Anfang, seine Halte, seine Richtungswechsel und sein Ende.

## Welche Halteplätze

<!-- cols: 26%, 1fr -->
| Zug | Wo er halten darf |
|---|---|
| Personenzug | nur an einem Halteplatz an einem Bahnsteig: einem, den ein Bahnsteig nennt (`stoppingPlaceIds`), einem, der als Personenhalt markiert ist (`passengerStop`), oder einem, neben dem auf demselben Gleis ein Bahnsteig liegt |
| Zug ohne Fahrgäste | an jedem Halteplatz |

- **Züge ohne Fahrgäste** sind die Gattungen Gz, Lz und Dz und die Güterzuggattungen GC, KLV, EK und DGS. Jede
  andere Gattung zählt als Personenzug.
- Hat ein Bahnhof keinen Halteplatz, den der Zug nutzen darf (auf dem verlangten Gleis), wird der Zug nicht
  gerechnet, und der Punkt sagt das.

## Wo im Halteplatz

- **10 m vor seinem Ende, in Fahrtrichtung gesehen.** An einem Halteplatz unter 20 m Länge hält der Zug in der Mitte.
  Am Anfang ist „das Ende“ das, auf das der Zug zufährt.
- Die Fahrtrichtung ergibt sich aus dem Weg, deshalb liefert jeder Halteplatz **zwei Kandidaten**, einen je Richtung.
- Von mehreren Kandidaten wird der auf dem kürzesten Weg erreichte genommen, am Anfang der mit dem kürzesten ersten
  Abschnitt. Ein anderer kommt nur dann in Frage, wenn vom ersten aus kein Weg weiterführt. Höchstens 200 Teilwege
  werden versucht.
- **Gerechnet wird die Zugspitze.** Ein haltender Zug steht mit der Spitze an diesem Punkt. Ob er dabei hinten über
  den Halteplatz oder den Bahnsteig hinausragt, wird nicht geprüft.

# Der Weg über das Netz

- **Kürzester Weg.** Der Weg von Punkt zu Punkt ist der nach Länge kürzeste. Gesucht wird nach Dijkstra über Paare
  aus Gleis und Fahrtrichtung, weiter über Weichen entlang ihrer Fahrwege, über Verbinder und Stöße. Die Kosten sind
  die gefahrenen Meter. Abzweige, Weichen und Geschwindigkeiten kosten nichts zusätzlich.
- **Jedes Gleis in beide Richtungen.** Der Weg fragt nicht, ob ein Gleis für seine Richtung signalisiert ist.
- **Kein Richtungswechsel, außer er ist verlangt.** Ab einem Halt fährt der Zug in der Richtung weiter, in der er
  ankam.
- **Bevorzugter Fahrweg.** Nennt ein Punkt einen Fahrweg der Leit- und Sicherungstechnik (`itineraryId`), fährt der
  Zug entlang dessen Gleisen, sofern es ihn in der Variante gibt und er hinführt. Sonst fährt er den kürzesten Weg und
  meldet das.
- **Umleitung durch die Disposition.** Schickt die Disposition einen Zug über eine andere Fahrstraße (siehe
  „Disposition“), fährt er zu deren Startsignal, entlang ihrer Gleise zum Zielsignal und von dort auf dem kürzesten
  Weg weiter.
- **Gesperrte Stücke.** In einem Bauszenario führt kein Weg um mehr als 0,5 m über ein gesperrtes Gleisstück (siehe
  „Bauszenarien“). Das gilt für den kürzesten Weg, für einen bevorzugten Fahrweg und für eine Fahrstraße, über die
  die Disposition einen Zug schickt.

## Richtungswechsel

- Ein Punkt unterwegs kann einen Richtungswechsel verlangen (`reverse`). Am ersten und letzten Punkt gibt es keinen.
- Der Zug **hält dort** (an einem Halteplatz, wie bei jedem Halt), steht seine **Wendezeit** (`turnaround`, ohne
  Angabe 300 s; die Haltezeit zählt dann nicht) und fährt in Gegenrichtung weiter.
- **Seine neue Spitze ist dort, wo sein Schluss stand,** eine Zuglänge zurück auf dem Weg, auf dem er kam. Der Weg
  springt deshalb an dieser Stelle, und das Stück, auf dem der Zug stand, wird nicht noch einmal befahren.
- Die Fahrzeitrechnung sieht die ganze Fahrt als einen Weg. Für die erste Zuglänge nach dem Richtungswechsel gelten
  deshalb als Neigung unter dem Zug und als Grenzen, die der Zugschluss räumen muss, die des Stücks, wie der Zug
  hineingefahren ist. Die signalisierten Geschwindigkeiten werden auf jedem Teil für sich gesucht.
- Wie die Blöcke am Richtungswechsel getrennt werden, steht unter „Sperrzeiten“.

# Neigungen

## Aus den Höhen

- Die Höhe eines Gleises liegt **linear zwischen seinen Höhenpunkten**. **Ausrundungen** (Neigungswechsel mit
  Halbmesser) gehen nicht in die Neigung ein: Am Höhenpunkt springt die Neigung von der einen Tangente auf die
  andere.
- Die Neigung eines Stücks zwischen zwei Schnittstellen ist `(z(b) − z(a)) / (b − a) · 1000` in ‰, mit dem Vorzeichen
  der Fahrtrichtung. Schnittstellen sind die Höhenpunkte, die Elementgrenzen und die Enden der Stücke des Wegs.
- **Ein Gleis ohne Höhen** (weniger als zwei Höhenpunkte) gilt als eben und wird als Lücke gemeldet.

## Bögen als Steigung

Der Bogenwiderstand wird als **gleichwertige Steigung** zur Neigung addiert, nach der Formel von **Röckl** für
Regelspur:

<!-- cols: 30%, 1fr -->
| Halbmesser r | Bogenwiderstand w in ‰ |
|---|---|
| r ≥ 300 m | `650 / (r − 55)` |
| 30 m < r < 300 m | `500 / (r − 30)` |
| r ≤ 30 m, Gerade | 0 |

- Ein **Kreisbogen** hat seinen Widerstand auf ganzer Länge.
- Ein **Übergangsbogen** wird in Stücke von höchstens 10 m geteilt, jedes mit dem Halbmesser in seiner Mitte. Die
  Krümmung verläuft bei der Klothoide linear von `1/r1` nach `1/r2`, beim Bloss-Bogen nach `3t² − 2t³`.
- Der Widerstand zählt **in beide Richtungen** als Steigung: Im Gefälle verringert er das wirksame Gefälle.
- Die Überhöhung spielt für den Widerstand keine Rolle. Spurweite und Achsstand des Fahrzeugs gehen nicht ein.
- Weichen haben ihren Bogenwiderstand, soweit ihr Zweig als Gleis mit Bogenelementen gespeichert ist.
- Die Summe wird auf 0,001 ‰ gerundet, gleiche Nachbarwerte werden zusammengefasst.

**Beispiel:** Ein Bogen mit r = 500 m auf einer Steigung von 4 ‰ ergibt 4 + 650 / 445 = 5,461 ‰.

## Unter dem Zug gemittelt

Die Fahrzeitrechnung nimmt nicht die Neigung unter der Spitze, sondern die **mittlere Neigung unter dem ganzen Zug**,
von Schluss bis Spitze. Der Zug gilt dabei als gleichmäßig über seine Länge verteiltes Band. Vor dem Anfang des
Wegs wird der Schluss am Wegbeginn abgeschnitten. Die Hangabtriebskraft ist `G = −m · g · sin(arctan(i / 1000))` mit
g = 9,81 m/s². Sie wirkt beim Beschleunigen, Halten, Rollen und Bremsen gleichermaßen.

# Geschwindigkeiten

## Was die Strecke zulässt

- **Entwurfsgeschwindigkeit:** die Geschwindigkeit eines Trassierungselements (`speed`). Ein Element ohne
  Geschwindigkeit setzt keine Grenze, und das Stück wird als Lücke gemeldet, sofern dort kein Signal eine
  Geschwindigkeit signalisiert.
- **Signalisierte Geschwindigkeit:** Für jede Zugfahrstraße, die der Weg entlangfährt, gilt die Geschwindigkeit, die
  ein logisches Signal an ihrem Startsignal für sie signalisiert (`signallingSpeeds`), vom Start- bis zum Zielsignal.
  Dafür müssen Start- und Zielsignal auf dem Weg stehen, ihre Gleise im Weg aufeinander folgen und das Startsignal in
  Fahrtrichtung wirken. Beginnt die Fahrstraße an einem fiktiven Signal an einer Betrachtungsgrenze (höchstens 20 m
  davon entfernt), gilt die Geschwindigkeit der Grenze.
- **Lf 7:** Steht auf der Fahrstraße eine Geschwindigkeitstafel Lf 7, die in Fahrtrichtung oder in beide Richtungen
  gilt, endet die Geschwindigkeit der Fahrstraße an ihr. Ab dort gilt die angezeigte Geschwindigkeit bis zur nächsten
  Lf 7 oder zum Zielsignal (Symbol `7` = 70 km/h).
- **Vorrang:** Wo ein Signal eine Geschwindigkeit signalisiert, **gilt sie anstelle der Entwurfsgeschwindigkeit**,
  auch wenn sie höher ist. Die Entwurfsgeschwindigkeit gilt nur, wo kein Signal spricht.
- **Weichengeschwindigkeiten** gehen nicht unmittelbar ein. Sie erreichen die Fahrt über die signalisierte
  Geschwindigkeit der Fahrstraße, die das Signalling-Tool aus ihnen vorschlägt.
- **Fahrzeug:** die Höchstgeschwindigkeit des Fahrzeugs gilt überall.
- **Ausfahrt über eine Grenze:** Der letzte Meter des Wegs trägt die Geschwindigkeit der Grenze.

## Wie der Zug ihnen folgt

- Aus allen Grenzen wird das **strengste Geschwindigkeitsprofil** gebildet (MRSP).
- **Eine Grenze gilt, bis der Zugschluss sie verlassen hat.** Hinter einer Langsamfahrstelle beschleunigt der Zug
  also erst, wenn er sie mit ganzer Länge verlassen hat.
- **Vor einer niedrigeren Grenze bremst der Zug so**, dass die Spitze sie an ihrem Beginn erreicht. Die Bremskurve
  hängt von der Neigung ab (siehe „Bremsen“).
- Der Fahrer gilt als **ideal**: Er bremst im letztmöglichen Augenblick, beschleunigt mit voller Zugkraft und hat
  keine Reaktionszeit. Ankündigungen (Lf 6, Zs 3v, Vorsignale) brauchen keine eigene Wirkung, denn der Bremseinsatz
  folgt aus der Physik.
- **Einfahrgeschwindigkeit** an einer Grenze: die der Grenze, höchstens die Höchstgeschwindigkeit des Zuges und
  höchstens so schnell, dass er vor jeder Grenze im Weg noch auf sie herunterbremsen kann: `√(v² + 2 · 0,8 · γ · s)`
  mit γ der Bremsverzögerung des Fahrzeugs und s dem Abstand bis zur Grenze. Der Faktor 0,8 lässt Raum für ein
  Gefälle.
- Das angezeigte Geschwindigkeitsprofil ist das ohne Zuglänge.

# Der Zug

## Fahrzeug

Ein Fahrzeug ist Rollmaterial im RailJSON von OSRD. Die Simulation liest daraus:

<!-- cols: 30%, 1fr -->
| Wert | Bedeutung |
|---|---|
| `mass` | Masse in kg |
| `length` | Länge in m (für das Mitteln der Neigung, die Grenzen, das Räumen der Blöcke und den Richtungswechsel) |
| `max_speed` | Höchstgeschwindigkeit in m/s |
| `inertia_coefficient` | Massenfaktor ρ für die drehenden Massen, sonst 1,05 |
| `rolling_resistance` | Laufwiderstand nach Davis: `A + B · v + C · v²` in N, v in m/s |
| `const_gamma` | Betriebsbremsverzögerung γ in der Ebene in m/s² |
| `effort_curves` | Zugkraft über Geschwindigkeit, je Traktionsart und Komfort |

- **Zugkraftkurve:** zwischen ihren Punkten linear. Unter dem ersten Punkt gilt dessen Zugkraft, über dem letzten
  ebenfalls dessen. Gewählt wird die Kurve für den Komfort des Zuges, sonst die für den Standardkomfort, sonst die
  Vorgabekurve der Traktionsart.
- **Ein Fahrzeug je Zug.** Gerechnet wird eine Zugbildung aus genau einem Triebfahrzeug. Zugbildungen aus mehreren
  Fahrzeugen, eine Wagenzuglast, Bremsstellung und Bremshundertstel werden gespeichert, aber nicht gerechnet. Ein Zug
  mit einer solchen Zugbildung lässt sich noch nicht fahren.
- Ein Laufwiderstand, dessen Beiwerte alle unter eins liegen, wird beim Einlesen als N/kg gelesen und mit der Masse
  multipliziert. Die Beispielfahrzeuge von OSRD sind so angegeben.
- Kein Haftwert: Die Zugkraft ist nicht durch die Reibung zwischen Rad und Schiene begrenzt. Nebenverbraucher und
  Leistungsgrenzen der Oberleitung zählen nicht.

## Traktionsart

- Die Traktionsart ist die des Zuges, wo er eine nennt.
- Sonst, wo die Oberleitung geplant ist: die elektrische Traktionsart des Fahrzeugs, deren Spannung den **ganzen Weg**
  unter Fahrdraht hat; sonst die thermische; sonst die Vorgabe des Fahrzeugs. Ist im Netz gar kein Fahrdraht geplant,
  fährt jedes Fahrzeug in seiner Vorgabe.
- **Eine Traktionsart für den ganzen Weg.** Einen Wechsel unterwegs gibt es nicht. Fährt ein Zug elektrisch über ein
  Stück ohne passenden Fahrdraht, fährt er dort trotzdem, und die Stücke werden gemeldet. Das gilt auch für eine
  Umleitung.
- Unter Fahrdraht liegt, was die Tragpunkte und Streckentrenner eines Fahrdrahts tragen. Die Spannung des Fahrdrahts
  muss gleich dem Namen der Traktionsart sein (`15000V`).

# Die Fahrzeitrechnung

## Bewegungsgleichung

Der Zug ist ein Massenpunkt an seiner Spitze, auf den drei Kräfte wirken: die Zugkraft F, der Laufwiderstand R(v) und
die Hangabtriebskraft G aus der mittleren Neigung mit den Bögen. Seine Beschleunigung ist

```
a = (F + G − R(v)) / (m · ρ)
```

- **Zeitschritt** 2 s. Die mittlere Beschleunigung eines Schritts wird nach Runge-Kutta geschätzt, dann folgt ein
  Newton-Schritt `Δx = v · Δt + a · Δt² / 2`. Ein Schritt, der unter null fallen würde, wird gekürzt.
- **Beschleunigen:** volle Zugkraft.
- **Halten:** gerade so viel Zugkraft, wie Widerstand und Neigung verlangen. Reicht die volle Zugkraft nicht, wird
  der Zug langsamer und holt danach wieder auf.
- **Rollen:** ohne Zugkraft.
- Im Stand bleibt der Zug stehen, solange Zugkraft und Hangabtrieb den Widerstand nicht überwinden. Bleibt ein Zug auf
  einer Steigung liegen, bricht die Rechnung mit `impossible_simulation` und der Stelle ab.

## Bremsen

Der Zug bremst mit der Betriebsbremsverzögerung γ des Fahrzeugs in der Ebene, **zuzüglich der Neigung unter dem
Zug**:

```
a = −γ + G / (m · ρ)
```

- G ist die Hangabtriebskraft wie beim Beschleunigen, unter dem Zug gemittelt, mit den Bögen. Eine Steigung verkürzt
  den Bremsweg, ein Gefälle verlängert ihn.
- **Beispiel:** Mit γ = 0,6 m/s² aus 100 km/h braucht der Zug in der Ebene etwa 643 m, in einem Gefälle von 10 ‰ etwa
  760 m.
- **In jedem Gefälle bleibt mindestens ein Zehntel von γ**, damit ein Zug immer zum Stehen kommt.
- Von der Geschwindigkeit hängt die Verzögerung nicht ab. Der Laufwiderstand hilft beim Bremsen nicht mit.
- OSRDs konstante Bremsung, die die Neigung nicht beachtet (`a = −γ`), bleibt in der Rechnung wählbar (`braking`
  `constant`); die Suite nutzt sie nicht.

## Die drei Fahrten

<!-- cols: 18%, 1fr -->
| Fahrt | Was sie ist |
|---|---|
| `base` | die schnellste Fahrt: mit voller Zugkraft beschleunigen, die Grenze halten, im letzten Augenblick bremsen |
| `provisional` | die Fahrt mit dem Regelzuschlag des Zuges |
| `final` | die Fahrt, die auch vorgegebene Ankunftszeiten hält; der Fahrplan nimmt diese |

Die Suite gibt der Rechnung keine Ankunftszeiten vor, nur Halte mit der Zeit, die der Zug steht. `final` ist daher die
Fahrt mit Zuschlag. Die Abfahrtszeiten des Fahrplans wirken erst danach (siehe „Uhr“).

## Zuschlag

- Ein Zug hat einen Fahrzeitzuschlag: in Prozent der Fahrzeit (`5 %`), in Minuten je 100 km (`4 min/100km`) oder
  keinen. Ein neuer Zug hat 5 % nach MARECO.
- Der Zuschlag gilt **für den ganzen Weg** als ein Bereich. Die Abschnitte zwischen den Halten werden für sich
  gerechnet.
- **MARECO** (Vorgabe) verteilt den Zuschlag energiesparend: Die Höchstgeschwindigkeit wird gedeckelt, und vor dem
  Bremsen rollt der Zug. Deckel und Rollbeginn werden durch Intervallhalbierung gesucht, bis die Zielzeit erreicht ist.
  Unter 1 m/s wird der Zug nicht verlangsamt.
- **Linear** verlangsamt alle Geschwindigkeiten im gleichen Verhältnis.

## Halte

- Ein Zug hält an jedem Punkt, der einen Halt oder einen Richtungswechsel verlangt, und immer an seinem Ende, außer
  er fährt dort über eine Grenze aus. Er bremst an der Haltestelle auf null (siehe „Wo ein Zug hält“).
- Er steht die **Haltezeit** (`dwell`, bei einem neuen Punkt 60 s) oder beim Richtungswechsel die **Wendezeit**
  (`turnaround`, ohne Angabe 300 s) und fährt mit voller Zugkraft wieder an.
- Zwei Halte an derselben Stelle werden einer, mit der längeren Zeit.
- Die Mindesthaltezeit eines Punktes (`leastDwell`) wird gespeichert, aber noch nicht gerechnet.
- Die Rechnung gibt die mechanische Energie am Rad aus, ohne Rückspeisung und ohne Wirkungsgrade.

# Uhr

- Ein Zug fährt zur Abfahrtszeit seines ersten Punkts ab.
- **Abfahrtszeit an einem Punkt:** Käme der Zug früher weg, wartet er bis zu ihr. Der Rest seiner Fahrt verschiebt sich
  um die Wartezeit. Neu gerechnet wird dafür nicht, denn ein längerer Halt ändert die Fahrt nicht. Das gilt auch am
  Richtungswechsel: Die Wendezeit ist das Mindeste, was er dort steht.
- Für jeden Ort des Wegs gibt es Ankunft und Abfahrt. Bei einer Durchfahrt sind beide gleich.
- Uhrzeiten über 24:00 gehören zum nächsten Tag und werden nicht umgebrochen.

# Signale

- **Hauptsignal** ist ein reales Signal mit einem logischen Signal, das kein Vorsignal, Vorsignalwiederholer und kein
  Sperrsignal (Rangiersignal) ist. Nur Hauptsignale begrenzen Blöcke.
- **Ankündigend** ist ein reales Signal mit einem Vorsignal, Vorsignalwiederholer oder Mehrabschnittssignal (Ks). Es
  bestimmt, wo ein Zug den nächsten Block anfordert.
- Ein Signal wirkt in seiner Richtung (`at.direction`, ohne Angabe in Stationierungsrichtung) oder in beide. Es kann an
  mehreren Stellen stehen (`alsoAt`).
- **Signale steuern die Fahrt nicht über Begriffe.** Die Simulation stellt keine Signalbegriffe (Hp 0, Ks 2) und bildet
  kein Fahrerverhalten an Signalen nach. Ein Signal wirkt auf die Fahrt nur auf zwei Wegen: über die Geschwindigkeit,
  die es signalisiert, und über die Disposition, die einen Zug vor ihm warten lässt.
- **Warten an einem Signal** ist ein geplanter Halt einige Meter davor. Der Zug bremst wie vor jedem Halt, nicht schon
  ab dem Vorsignal, und fährt nach der Wartezeit mit voller Zugkraft an.
- **Fiktive Signale** an Betrachtungsgrenzen beginnen oder beenden Fahrstraßen ohne reales Signal.
- Die **Sichtweite** eines ankündigenden Signals ist `sight.required`, sonst `sight.minimum`.

# Blöcke

- **Ein Block** ist das Stück des Wegs eines Zuges von einem Hauptsignal, das in seiner Fahrtrichtung wirkt, bis zum
  nächsten. Der erste Block beginnt am Start, der letzte endet am Ziel; ein Richtungswechsel beendet einen Block und
  beginnt den nächsten. Ohne Hauptsignale ist der ganze Weg ein Block.
- **Durchrutschweg:** Ein Block reicht über sein Endsignal hinaus um den längsten der Durchrutschwege, die für dieses
  Signal gespeichert sind. Die Simulation rechnet ihre Länge nicht selbst. Der Katalog nach Ril 301 schlägt sie im
  Signalling-Tool vor: 50 m bis 40 km/h, 100 m bis 60 km/h, darüber 200 m; in einem Gefälle steiler als 5 ‰ länger
  (`L = L0 · 0,75 / (0,75 − 9,81 · |i| / 1000)`, auf 10 m aufgerundet). Diese Tabelle ist ein Entwurf und nicht gegen
  die gültige Ril 819.0202 geprüft.
- **Gleisfreimeldung:** Hat die Variante Freimeldeabschnitte, hält ein Block jeden Abschnitt, den er auf mehr als
  0,5 m berührt. Die Ausdehnung der Abschnitte folgt aus ihren Freimeldepunkten über das Netz. Stücke, die kein
  Abschnitt deckt, hält der Block als Gleisstück.
- **Konflikt:** Zwei Züge stehen im Konflikt, wo sie zur selben Zeit denselben Abschnitt halten oder Stücke desselben
  Gleises, die sich um mehr als 0,5 m überdecken.
- **Fahrstraßenzeiten:** Bilde- und Auflösezeit eines Blocks sind die der Zugfahrstraße von seinem Signal zum nächsten,
  sonst die des Layers (`routeTimes`), sonst die des Fahrplans (`blockingTimes`), sonst 12 s und 6 s.

# Sperrzeiten

Ein Zug hält einen Block von dem Augenblick, in dem er ihn frei vorfinden muss, bis der Block nach seinem Durchfahren
aufgelöst ist. Das folgt der Sperrzeit nach Ril 405 und den Belegungsanforderungen von OSRD:

<!-- cols: 26%, 1fr -->
| Anteil | Wie er gerechnet wird |
|---|---|
| Fahrstraßenbildezeit | vor dem Anfordern; 12 s, wo nichts anderes gilt |
| Annäherung | ab dem Punkt, an dem der Zug das ankündigende Signal sieht: dessen Sichtweite vor dem ersten ankündigenden Signal seit dem vorigen Hauptsignal; ohne ankündigendes Signal 1000 m vor dem Signal des Blocks |
| Fahrt im Block | wie gerechnet, mit Zuschlag und Halten |
| Räumen | bis der Zugschluss das Endsignal des Blocks passiert hat (Endsignal + Zuglänge) |
| Auflösezeit | danach; 6 s, wo nichts anderes gilt |

- Die **Annäherungsfahrzeit** ist die der wirklichen Fahrt des Zuges. Eine Sichtzeit in Sekunden (`sight` in
  `blockingTimes`) wird gespeichert, aber noch nicht gerechnet.
- **Erster Block:** ab der Abfahrt, abzüglich der Bildezeit. **Halt in der Annäherung:** Steht der Zug innerhalb der
  Annäherung, fordert er den Block erst bei seiner Abfahrt dort an.
- **Letzter Block:** bis zur Ankunft am Ziel und der Auflösezeit.
- **Richtungswechsel:** Die Blöcke werden dort getrennt wie an einem Ende und einem Anfang.
  - Der Block davor endet am Wendepunkt, ohne Zielsignal und ohne Durchrutschweg. Er bleibt belegt, bis der Zug in
    Gegenrichtung das Stück verlassen hat, auf dem er stand (eine Zuglänge), dazu die Auflösezeit.
  - Der Block danach beginnt dort ohne Signal und wird bei der Abfahrt verlangt, abzüglich der Bildezeit.
  - Ein dort aufgehaltener Zug wartet am Wendepunkt und steht länger.
- **Teilfahrstraßenauflösung:** Mit Freimeldeabschnitten wird jeder Abschnitt aufgelöst, sobald der Zugschluss ihn
  verlassen hat, zuzüglich der Auflösezeit. Eine Fahrstraße wartet so nur auf den Abschnitt, den ein anderer noch
  hält.
- **Durchrutschweg:** Seine Abschnitte werden mit dem Block aufgelöst. Kommt der Zug innerhalb von 200 m vor dem
  Endsignal zum Stehen, werden sie schon eine Weile nach seinem Halt aufgelöst: nach der Auflösezeit des
  Durchrutschwegs (`releaseDelay`), sonst nach 60 s.

# Disposition

Ein Fahrplan sagt, was geschieht, wo sich Züge begegnen (`dispatching.mode`):

- **`show`** fährt jeden Zug für sich und zeigt die Konflikte, wie OSRD.
- **`wait`** (Vorgabe) stellt die Fahrstraßen der Züge in zeitlicher Folge, wie ein Stellwerk: Wer zuerst anfordert,
  bekommt zuerst. Fordern zwei im selben Augenblick an, entscheidet die Abfahrt oder die Reihenfolge der Gattungen
  (`priority`), danach die Zugnummer. Die Vorgabereihenfolge der Gattungen ist ICE, EC, IC, IRE, RE, RB, S, Dz, Gz, Lz.

Im Modus `wait` gilt:

- Ein Zug fordert seinen nächsten Block an, wenn er sich dessen Signal nähert, also zu Beginn der Sperrzeit. Er
  bekommt ihn, wenn keiner seiner Abschnitte von den Blöcken gehalten wird, die andere schon bekommen haben.
- Vom letzten Block eines anderen Zuges, der seinen nächsten noch nicht hat, gilt als gehalten, was dieser auch dann
  noch hielte, wenn er vor dem nächsten Signal zum Stehen käme. Der Durchrutschweg gilt nur bis eine Weile nach diesem
  Halt als gehalten.
- **Gehalten:** Der Zug nimmt eine andere Zugfahrstraße vom selben Signal, die frei ist und weiter zu seinem Ziel
  führt, etwa ins andere Bahnhofsgleis, um dort zu kreuzen. Gibt es keine, wartet er auf die Fahrstraße, die zuerst
  frei wird: länger an seinem letzten Halt zwischen dem vorigen Signal und dem Signal des Blocks, soweit der in der
  Annäherung liegt (sein Start zählt dazu, dann fährt er später ab); sonst 10 m vor dem Signal. Auf seinen ersten
  Block wartet er am Start.
- Ein Zug, der wartet oder umgeleitet wird, wird **neu gerechnet**: Die Wartezeit ist ein Halt, die Umleitung ein
  anderer Weg.
- **Grenzen:** Die Disposition trifft höchstens 80 Entscheidungen je Zug (für alle Züge zusammen, zuzüglich 50), und
  ein Zug fordert einen Block höchstens 20-mal neu an. Danach fahren die übrigen Züge, wie sie fahren würden, und was
  bleibt, wird als Konflikt gemeldet.
- **Verspätung** ist, wie viel später ein Zug ankommt als allein gefahren.
- **Zugfolge an einer Grenze:** Hat eine Betrachtungsgrenze eine Mindestzugfolgezeit (`headway`), wird ein Zug
  gemeldet, der dort früher nach dem vorigen einfährt. Er wird dafür nicht verschoben.
- Die Disposition optimiert nicht. Sie tauscht keine Reihenfolgen über „wer zuerst anfordert“ hinaus und hält keine
  Anschlüsse und keine Wenden von einem Zug in einen anderen.

# Kapazität nach UIC 406

## Abschnitte und Verdichtung

- **Abschnitte:** Ein Abschnitt reicht von einem Knoten zum nächsten entlang der Wege der Züge, für beide Richtungen
  derselbe. Ein Knoten ist ein Bahnhof mit mindestens zwei Gleisen oder ein Ort, an dem ein Zug beginnt oder endet.
- **Züge eines Abschnitts** sind die, die von einem seiner Orte zum anderen fahren, mit ihrer Zeit an seinem Anfang
  und den Blöcken, die mehr als 1 m in ihn hineinreichen.
- **Verdichtung:** Die Züge, deren Zeit am Anfang des Abschnitts im Zeitraum `[von, bis)` liegt, werden in der
  Reihenfolge, in der sie dort fahren, so früh hinter die vorigen gelegt, wie ihre Sperrzeiten es erlauben, nie vor den
  Zug davor. Ein Abschnitt oder Gleisstück, das ein Block vor seinem Ende freigibt, zählt nur bis dahin.
- **Belegungszeit:** vom ersten begonnenen bis zum letzten beendeten Block der verdichteten Züge. Der **Belegungsgrad**
  ist ihr Anteil am Zeitraum, mit und ohne Zuschläge (je Zug und fest).
- **Empfohlene Grenzen** nach UIC 406 (2013): Für einen Zeitraum bis zwei Stunden gilt der Wert der Spitzenstunde,
  sonst der Tageswert.

<!-- cols: 34%, 22%, 1fr -->
| Art der Strecke | Spitzenstunde | Tag |
|---|---|---|
| S-Bahn | 85 % | 70 % |
| Hochgeschwindigkeit | 75 % | 60 % |
| Mischverkehr | 75 % | 60 % |

- **Mindestzugfolgezeit:** für je zwei Züge, die etwas gemeinsam halten, in beiden Reihenfolgen, wie früh der zweite
  nach dem ersten am Anfang sein darf, ohne dass sich ihre Sperrzeiten überdecken. Die Werte werden nach Gattungen
  zusammengefasst.

## Ausschlaggebende Belegungselemente und Konfliktbereiche

- **Die kritische Kette** sind die verdichteten Züge hintereinander: Jeder liegt so früh, wie der davor es zulässt.
- **Ausschlaggebendes Belegungselement:** Für jeden Zug der Kette nach dem ersten das Element, an dem seine Sperrzeit
  die des Zuges berührt, an den er gebunden ist. Von allem, was beide belegen, ist es das, an dem er am wenigsten
  früher kommen könnte. Es wird mit dem Block beider Züge angegeben und mit dem Augenblick, in dem der eine es
  freigibt und der andere es belegt, wie verdichtet. Ein Element ist ein Freimeldeabschnitt oder, wo es keine gibt,
  ein Gleisstück.
- **Konfliktbereich:** was zwei oder mehr Züge des Zeitraums belegen. Das ist ein Freimeldeabschnitt oder ein
  Gleisstück, in dem sich die belegten Stücke um mehr als 0,5 m überlappen, als eines genommen. Zu jedem Bereich
  gehören die Züge, die ihn belegen, und die Vereinigung der Zeiten, in denen er belegt ist.
- **Im Lageplan:** Die Kapazitäts-Sim zeichnet die Bereiche und die ausschlaggebenden Elemente in einen schematischen
  Lageplan der Gleise der Variante: jeden Bereich gefärbt nach der Zahl der Züge, die ihn belegen, die
  ausschlaggebenden Elemente nummeriert.

## Bahnhöfe und Bauzustände

- **Bahnhöfe:** Für jedes Gleis und jeden Abschnitt eines Bahnhofs zählt die Vereinigung der Zeiten, in denen er
  gehalten wird, und ihr Anteil am Zeitraum.
- **Bauzustände:** Die Kapazität wird auf dem Netz einer Variante an einem Bauzustand gerechnet: dem Bestand, jedem
  Tag, an dem etwas in Betrieb geht oder außer Betrieb kommt, oder dem Endzustand. Der Stand an einem Bauzustand
  folgt aus den Lebensdauern; eine andere Variante oder ein anderer Bauzustand lässt sich daneben legen.

# Bauszenarien

Ein Bauszenario gehört zu einem Fahrplan (Entscheidung 0014; `constructionScenarios` in timetables 1.2). Es sperrt
Gleisstücke eine Zeit lang, nimmt Züge heraus und leitet Züge von Hand um. Die Baubetriebs-Sim fährt den Fahrplan
darin für einen Tag und stellt jeden Zug neben seine Regelfahrt.

## Das Netz des Tages

- **Was es gibt,** ist die Variante im Bauzustand dieses Tages, wie die Lebensdauern aus dem Layout-Tool sie
  ergeben. Ein Gleis, das noch nicht oder nicht mehr in Betrieb ist, gibt es nicht; ein Szenario muss es nicht
  sperren.
- **Die Regelfahrten** sind die Züge des Fahrplans auf diesem Netz, disponiert wie im Fahrplan-Editor, ohne jede
  Sperrung.

## Sperrungen

- **Eine Sperrung** sperrt ein Stück eines Gleises, von einer Stationierung bis zu einer anderen.
- **Wann:** vom ersten bis zum letzten Tag, ganztägig oder in einem täglichen Zeitfenster (Sperrpause). Ein Fenster,
  dessen Ende vor seinem Beginn liegt, reicht über Mitternacht in den nächsten Tag. Eine Sperrung des Vortags reicht
  deshalb in die frühen Stunden des Tages; gilt sie auch am Folgetag, zählt sie für die Fahrten über
  Mitternacht.
- **Kein Weg führt über ein gesperrtes Stück**, nicht um mehr als 0,5 m. Der Rest des Gleises bleibt nutzbar: Ein
  Zug kann vor der Sperrung halten oder hinter ihr beginnen.

## Welche Züge betroffen sind

- **Ein Zug ist betroffen** von einer Sperrung, wo seine Regelfahrt über das gesperrte Stück führt, während die
  Sperrung gilt: von der Spitze, die in das Stück einfährt, bis zum Schluss, der es verlässt.
- Ein **nicht betroffener** Zug behält seinen Weg. Eine Sperrung in einem Zeitfenster sperrt ein Gleis nicht für die
  Züge außerhalb davon.

## Umleitung

- **Automatisch:** Ein betroffener Zug fährt den kürzesten Weg, mit dem gesperrt, was ihn betrifft, und behält seine
  Punkte: seine Halte, Richtungswechsel und Abfahrtszeiten. Wo der Umweg eine weitere Sperrung berührt, wird auch
  diese für ihn gesperrt, höchstens dreimal.
- **Von Hand:** Eine von Hand gesetzte Umleitung fährt den Zug auf ihren eigenen Punkten — Weg, Halte,
  Richtungswechsel, Fahrwege — und geht der gerechneten vor. Was sie berührt, wird für ihn gesperrt wie für jeden.
- **Nicht fahrbar:** Ein Zug, für den kein Weg bleibt, sagt, an welchem Punkt er keinen fand. Ein Punkt auf einem
  gesperrten Stück oder einer, der nur darüber erreichbar ist, lässt sich nicht halten; eine Umleitung von Hand ohne
  ihn schon.
- **Herausgenommen:** Ein herausgenommener Zug fährt nicht und belegt nichts.

## Das Szenario fahren

- Die fahrenden Züge, die regulären eingeschlossen, bekommen das Netz von der Disposition wie im Fahrplan-Editor
  (siehe „Disposition“), jeder auf seinem Netz mit dem, was für ihn gesperrt ist. Ein umgeleiteter Zug kann deshalb
  einen nicht betroffenen aufhalten.
- Jeder Zug endet in einem Zustand:

<!-- cols: 22%, 1fr -->
| Zustand | Bedeutung |
|---|---|
| regulär | nicht betroffen; fährt seinen Weg, durch die Disposition vielleicht später |
| umgeleitet | betroffen; fährt den für ihn gerechneten Umweg |
| von Hand | fährt die von Hand gesetzte Umleitung |
| nicht fahrbar | es bleibt kein Weg; der Punkt, an dem keiner gefunden wurde |
| herausgenommen | fährt nicht |

- **Neben dem Fahrplan:** Für jeden Zug seine Ankunft im Szenario und regulär, die Änderung seiner Fahrzeit, seine
  Verspätung (wie viel später er ankommt als in seiner Regelfahrt), seine Wartezeiten und seine Konflikte.

# Grenzen

Nicht gerechnet werden, teils weil OSRD-Teile nicht portiert sind, teils weil die Suite sie noch nicht nutzt:

- Bremskurven von ETCS und LZB, Fahrerverhalten an Signalen, Signalbegriffe, die Signalisierungssimulation;
- Wechsel der Traktionsart unterwegs, elektrische Profile, Leistungsgrenzen, Schutzstrecken;
- Zugbildungen aus mehreren Fahrzeugen, Wagenzuglast, Bremsstellung und Bremshundertstel;
- Ausrundungen in der Neigung, eine Bremsverzögerung abhängig von der Geschwindigkeit, der Haftwert;
- ob das Ende eines haltenden Zuges noch in den Halteplatz oder an den Bahnsteig passt;
- ob ein Gleis in einer Richtung befahren werden darf (Gleiswechselbetrieb);
- Anschlüsse, Wenden von einem Zug in einen anderen, Verspätungsszenarien und stochastische Läufe einer
  Untersuchung (`runs`, `seed`) — sie werden gespeichert;
- die Mindesthaltezeit (`leastDwell`) und die Sichtzeit (`sight`);
- vorgegebene Ankunftszeiten in der Fahrzeitrechnung;
- im Bauszenario: Langsamfahrstellen an der Baustelle, Bauzüge und ob eine Umleitung über Weg, Traktion und Halte
  hinaus für den Zug geeignet ist.

# Vorgabewerte

<!-- cols: 42%, 18%, 1fr -->
| Wert | Vorgabe | Wo er sich ändern lässt |
|---|---|---|
| Zeitschritt der Integration | 2 s | — |
| Massenfaktor | 1,05 | Fahrzeug `inertia_coefficient` |
| Erdbeschleunigung | 9,81 m/s² | — |
| Bremsverzögerung, die in jedem Gefälle bleibt | ein Zehntel von γ | — |
| Fahrzeitzuschlag eines neuen Zuges | 5 %, MARECO | Zug |
| Haltezeit eines neuen Punktes | 60 s | Punkt `dwell` |
| Wendezeit beim Richtungswechsel | 300 s | Punkt `turnaround` |
| Halt vor dem Ende eines Halteplatzes | 10 m, in Fahrtrichtung | — |
| Halteplatz, an dem in der Mitte gehalten wird | kürzer als 20 m | — |
| Fahrstraßenbildezeit | 12 s | Fahrstraße, Layer `routeTimes`, Fahrplan `blockingTimes.setup` |
| Fahrstraßenauflösezeit | 6 s | Fahrstraße, Layer `routeTimes`, Fahrplan `blockingTimes.release` |
| Annäherung ohne ankündigendes Signal | 1000 m | Fahrplan `blockingTimes.approach` |
| Auflösung des Durchrutschwegs nach Halt | 60 s | Durchrutschweg `releaseDelay` |
| Halt vor dem Endsignal, der den Durchrutschweg auflöst | 200 m | — |
| Warten vor einem Signal | 10 m davor | — |
| Gemeinsames Gleisstück für Konflikt, Bereich oder Sperrung | 0,5 m | — |
| Stücke eines Übergangsbogens | höchstens 10 m | — |
| Reserve der Einfahrgeschwindigkeit | 0,8 · γ | — |
| Teilwege bei der Gleiswahl | 200 | — |
| Entscheidungen der Disposition je Zug | 80 | — |
| Runden einer Umleitung | 3 | — |

# Wo es im Code steht

<!-- cols: 34%, 1fr -->
| Thema | Quelle |
|---|---|
| Orte | `packages/running-times/src/stations.js` |
| Wo ein Zug hält | `packages/timetables/src/timetable.js` (`routePoints`, `stoppingPlacesFor`, `stopsIn`) |
| Weg, gesperrte Stücke | `packages/running-times/src/path.js` (`findPath`, `pathAlong`, `closed`) |
| Richtungswechsel | `packages/running-times/src/route.js` (`routeVia`, `backAlong`), `packages/timetables/src/occupancy.js` (`blocksAlong`) |
| Neigungen, Bögen, Geschwindigkeiten | `packages/running-times/src/route.js`, `request.js` |
| Fahrzeug | `packages/vehicles/src/vehicles.js` |
| Traktionsart | `packages/running-times/src/traction.js` |
| Fahrzeitrechnung | `sim/crates/sim` (Physik, Hüllkurven, Zuschläge, eine Zugfahrt) |
| Bremsen auf der Neigung | `sim/crates/sim/src/physics/integrator.rs` (`braking_deceleration`) |
| Uhr, Fahrplanzeiten | `packages/timetables/src/timetable.js` |
| Blöcke, Sperrzeiten, Disposition | `packages/timetables/src/occupancy.js` |
| Durchrutschweg, Fahrstraßengeschwindigkeit | `packages/core/src/signalling/overlaps.js`, `packages/core/constraints/db-ril-301.json` |
| Freimeldeabschnitte | `crates/signalling/src/sections.rs` |
| Kapazität, ausschlaggebende Elemente, Konfliktbereiche | `crates/capacity`, `packages/capacity`, `apps/capacity-sim/src/ConflictPlan.jsx` |
| Bauzustände | `apps/construction-sim/src/model/stages.js`, `packages/core` (`recordAtStage`) |
| Bauszenarien | `packages/timetables/src/scenario.js`, `apps/construction-sim/src/Scenarios.jsx` |
