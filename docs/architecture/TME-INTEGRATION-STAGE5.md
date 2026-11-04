# TME Integration – Stage 5 (Codeentwurf, noch nicht kompiliert)

## Neu gegenüber Stage 4

- Persistente Akquisitionssitzungen können externe **Hinweise** als reine Quellreferenz und Grund dokumentieren (`ExternalAcquisitionHint`).
- Ein über die Soul/ToolGateway-Seite beschaffter Hinweis darf die Query-Einbettung für eine bereits laufende Sitzung ändern und damit vorhandene, noch nicht aktivierte Memory-Tokens reaktivieren. Das TME-Modul führt selbst **keinen** externen Netzwerkzugriff aus.
- Die Reaktivierung darf keine neuen Memory-Tokens anlegen; neue Evidenz müsste später gesondert über die bestehende Ingestion/Admission laufen.
- Die neue Bridge-Schnittstelle bietet `session-start`, `session-feedback`, `session-hint`; Ausgabe erfolgt nach validierter atomarer Container-Transaktion.
- Die Metadaten-Validierung prüft referenzielle Integrität, Grenzen und Vektoren persistenter Sitzungen und die Referenzen bestehender Abstraction-Records.
- Unit-Tests für Reaktivierung, Persistenz, fehlerhafte Hinweise und manipulierte Referenzen hinzugefügt.

## Kritische Grenzen

Nicht kompiliert oder ausgeführt in der Erstellungslaufzeit (hier ist kein Cargo vorhanden). Patch **nicht** auf produktiven `.tme`-States einsetzen. Nur in separater temporärer Academy-Testkopie nach `cargo test` und `cargo check --bin text-ingestion-bridge` validieren. Kein Code für externe Provider, Policy-Prüfung oder autonomes Tool-Calling. SLEEP führt bislang nur Assessments aus; sichere Zustandsbereinigung ist noch offen. Keine neue semantische Abstraction-Entscheidungsfunktion.

## CLI-Beispiele (nach erfolgreicher Kompilierung, nur isolierte Test-States)

`session-start`: JSON `{"session_id":"q1","anchor_id":"doc","query_vector":[...],"max_depth":3,"batch_size":2}`

`session-feedback`: JSON `{"session_id":"q1","feedback":"insufficient"}`

`session-hint`: JSON `{"session_id":"q1","query_vector":[...],"source_reference":"library:document-7","reason":"externer Hinweis"}`

Die API verwendet keine Schlüssel, keine externen Provider und keine Wallet-Funktionen.
