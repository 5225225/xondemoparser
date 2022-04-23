A demo is a header followed by a list of packets.

A header is a series of characters terminated by a newline (`\n`)

A demo is a list of packets.

A packet has the following format, common amongst all of them. All integers are little endian. ( so far,,,)

# Packet Format

## `length_and_direction: u32`

This field indicates the size of the packet. The most significant bit is set if this packet is client -> server, and unset if this packet is server -> client. The rest of the bits are the length of this packet (*not including the viewangles*)

## `viewangles: (f32, f32, f32)`

View angles. In `x`, `y`, `z` order.
