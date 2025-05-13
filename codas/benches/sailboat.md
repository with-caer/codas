# `Sailboat` Coda

Data structure used for benchmarking codecs in relation to Codas.

## `Sail` Data

The sail(s) of a sailboat.

+ `surface_area` f32

    Total surface area across all sails in square meters.

+ `sail_count` u8

    Number of distinct sails.

+ `breaking_strength` f32

    Ultimate tensile strength in kilograms per square meter.

+ `surface_material` text

    Name of the material used for the majority of the sails' surface.

+ `rigging_material` text

    Name of the material used for the rigging (aka rope) on the sails.

## `Hull` Data

The hull of a sailboat.

+ `manufacturer_id` text

    HID manufacturer identification code.

+ `serial_number` u32

    HID serial number.

+ `manufacture_year` text

    HID year of manufcature.

+ `model_year` text

    HID model year.

+ `length` f32

    Length of the hull from aft to stern in meters.

## `Boat` Data

A sailboat.

+ `name` text

    Unique name of the vessel.

+ `seaworthy` bool

    True iff the boat is currently ready to set sail.

+ `sail` Sail
+ `hull` Hull