/test/proc/test_byondapi_connection()
	var/ret = _bapidmm_test_connection()
	if (ret != 10)
		throw EXCEPTION("Connection bad")

/test/proc/test_dmm_parsing()
	var/datum/bapi_parsed_map/B = load_map("test_map.dmm", measure_only = TRUE)
	if(B.has_warnings())
		CRASH("warnings produced: [json_encode(B.loaded_warnings)]")
	ASSERT(B._internal_index != -1)
	ASSERT(B.original_path == "test_map.dmm")
	ASSERT(B.map_format == MAP_DMM)
	ASSERT(B.key_len == 1)
	ASSERT(B.line_len == 10)
	if(B.bounds ~! list(1, 1, 1, 10, 10, 1))
		CRASH("Expected bounds to be list(1, 1, 1, 10, 10, 1), but found [json_encode(B.bounds)]")

/test/proc/test_tgm_parsing()
	var/datum/bapi_parsed_map/B = load_map("test_map_tgm.dmm", measure_only = TRUE)
	if(B.has_warnings())
		CRASH("warnings produced: [json_encode(B.loaded_warnings)]")
	ASSERT(B._internal_index != -1)
	ASSERT(B.original_path == "test_map_tgm.dmm")
	ASSERT(B.map_format == MAP_TGM)
	ASSERT(B.key_len == 1)
	ASSERT(B.line_len == 1)
	if(B.bounds ~! list(1, 1, 1, 10, 10, 1))
		CRASH("Expected bounds to be list(1, 1, 1, 10, 10, 1), but found [json_encode(B.bounds)]")

/test/proc/test_loading()
	var/datum/bapi_parsed_map/B = load_map("load.dmm", 1, 1, 1)
	if(B.has_warnings())
		CRASH("warnings produced: [json_encode(B.loaded_warnings)]")
	ASSERT(B._internal_index != -1)
	var/count = 0
	for(var/obj/placed_at_runtime/O in world)
		count += 1
	ASSERT(count == 2)

/test/proc/test_loading_oob()
	var/before_bounds = _bapi_helper_get_world_bounds()
	var/datum/bapi_parsed_map/B = load_map("oob.dmm", 1, 1, 1, crop_map = TRUE)
	if(B.has_warnings())
		CRASH("warnings produced: [json_encode(B.loaded_warnings)]")
	var/after_bounds = _bapi_helper_get_world_bounds()
	ASSERT(before_bounds ~= after_bounds)
	ASSERT(B._internal_index != -1)
	var/count = 0
	for(var/obj/placed_at_runtime/O in world)
		count += 1
	if(count != 7)
		CRASH("Expected 7 placed_at_runtime objects, found [count]")

// Must be after test_loading_oob or count will be off
/test/proc/test_loading_oob_no_crop()
	var/before_bounds = _bapi_helper_get_world_bounds()
	var/datum/bapi_parsed_map/B = load_map("oob.dmm", 1, 1, 1, crop_map = FALSE)
	if(B.has_warnings())
		CRASH("warnings produced: [json_encode(B.loaded_warnings)]")
	var/after_bounds = _bapi_helper_get_world_bounds()
	ASSERT(before_bounds ~! after_bounds)
	ASSERT(B._internal_index != -1)
	var/count = 0
	for(var/obj/placed_at_runtime/O in world)
		count += 1
	if(count != 29)
		CRASH("Expected 29 placed_at_runtime objects, found [count]")

/test/proc/test_loading_modified_prefab()
	var/datum/bapi_parsed_map/B = load_map("prefab.dmm")
	if(B.has_warnings())
		CRASH("warnings produced: [json_encode(B.loaded_warnings)]")
	ASSERT(B._internal_index != -1)
	var/count = 0
	for(var/obj/modified/O in world)
		count += 1
		ASSERT(initial(O.name) == "hehe")
		ASSERT(O.name == "not_hehe")
	ASSERT(count == 2)

/test/proc/test_turf_and_area()
	var/datum/bapi_parsed_map/B = load_map("turf_and_area.dmm")
	if(B.has_warnings())
		CRASH("warnings produced: [json_encode(B.loaded_warnings)]")
	ASSERT(B._internal_index != -1)
	var/area/placed_at_runtime/A = locate()
	ASSERT(A != null)
	var/count = 0
	for(var/turf/placed_at_runtime/P in world)
		ASSERT(P.loc == A)
		count += 1
	ASSERT(count == 2)
	
	count = 0
	for(var/turf/template_noop/T in world)
		count += 1
	ASSERT(count == 0)

	count = 0
	for(var/area/template_noop/whatever in world)
		count += 1
	ASSERT(count == 0)

/test/proc/legacy_test()
	for(var/A in world)
		del(A)
	world.maxx = 0
	world.maxy = 0
	world.maxz = 0
	world.log << "Reset world to ([world.maxx], [world.maxy], [world.maxz])"

	var/initial_world_contents = ""
	for(var/atom/A in world)
		initial_world_contents += "[A.type]-[A.name]"

	if(initial_world_contents != "/area-area" && initial_world_contents != "") // one area left
		CRASH("Failed to clear previous tests: [initial_world_contents]")
	initial_world_contents = ""

	world.log << "meow!"

	var/datum/parsed_map/P = load_map_old(file("MetaStation-tgm.dmm"), 1, 1, 1, new_z = TRUE, no_changeturf = TRUE)
	world.log << "old thinks it is: [P.original_path] ([P.map_format])"
	world.log << "bounds of old: [json_encode(P.bounds)]"
	world.log << "old model cache (len): [length(P.modelCache)]"
	world.log << "old thinks it expanded x? [P.expanded_x] y? [P.expanded_y]"
	world.log << "old skipped turfs: [P.turfsSkipped]"
	world.log << "world xyz rn [world.maxx] [world.maxy] [world.maxz]"

	var/list/old_world_contents = list()
	for(var/atom/A in world)
		old_world_contents["[A.type]"]++

	world.log << "types: [length(old_world_contents)]"

	// clean up our areas
	areas_by_type = list()
	for(var/A in world)
		del(A)

	world.maxx = 0
	world.maxy = 0
	world.maxz = 0

	for(var/atom/A in world)
		initial_world_contents += "[A.type]-[A.name]"

	if(initial_world_contents != "/area-area") // one area left
		CRASH("Failed to clear previous tests: [initial_world_contents]")

	var/datum/bapi_parsed_map/B = load_map("MetaStation-tgm.dmm", 1, 1, 1, new_z = TRUE, no_changeturf = TRUE)
	if(B.has_warnings())
		CRASH("warnings produced: [json_encode(B.loaded_warnings)]")
	ASSERT(B._internal_index != -1)
	world.log << "meta-tgm internal index [B._internal_index]"
	world.log << "bounds of bapi: [json_encode(B.bounds)]"

	var/list/world_contents = list()
	for(var/atom/A in world)
		world_contents["[A.type]"]++

	world.log << "bapi types: [length(world_contents)]"
	world.log << "world xyz rn [world.maxx] [world.maxy] [world.maxz]"

	for(var/type in world_contents)
		if(!(type in old_world_contents))
			stack_trace("BAPIDMM differed from DMMREADER: [type] was produced by BAPI, but not DMMREADER")
			continue
		var/count = world_contents[type]
		var/old_count = old_world_contents[type]
		if(count != old_count)
			stack_trace("BAPIDMM differed from DMMREADER: [type] was spawned [count] times by BAPI, but [old_count] times by DMMREADER")

	for(var/type in old_world_contents)
		if(!(type in world_contents))
			stack_trace("BAPIDMM differed from DMMREADER: [type] was not produced by BAPI")
			continue

	if(B.bounds ~! P.bounds)
		stack_trace("BAPIDMM differed from DMMREADER: BAPI calced bounds as [json_encode(B.bounds)] but DMMREADER calced as [json_encode(P.bounds)]")