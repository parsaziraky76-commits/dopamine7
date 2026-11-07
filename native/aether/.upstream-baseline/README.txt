Pristine upstream copies of the files this app patches, at core 1.9.0.

Do not edit. scripts/sync-core.sh uses them as the merge base so the app's
engine patches can be rebased onto a new core instead of overwriting it.

1.2.4: masque_h2.rs left this set — upstream 1.9.0 rewrote the HTTP/2 send path
itself (pump_outbound + append_datagram_capsule), which is what our patch did,
so keeping ours would have meant maintaining a fork of code upstream now owns.
The patches that stayed carry AETHER-APP-PATCH markers in the source.
