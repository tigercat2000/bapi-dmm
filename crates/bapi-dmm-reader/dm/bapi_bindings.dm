//THIS FILE IS AUTOMATICALLY GENERATED BY BAPI_DMM_READER, PLEASE DO NOT TOUCH IT
//PROC DEFINITIONS MAY MOVE AROUND, THIS IS NORMAL

/* This comment bypasses grep checks */ /var/__bapi_dmm_reader

/proc/__detect_bapi_dmm_reader()
	if (world.system_type == UNIX)
		return __bapi_dmm_reader = "libbapi_dmm_reader"
	else
		return __bapi_dmm_reader = "bapi_dmm_reader"

#define BAPI_DMM_READER (__bapi_dmm_reader || __detect_bapi_dmm_reader())
    
/proc/_bapidmm_clear_map_data()
	return call_ext(BAPI_DMM_READER, "byond:_bapidmm_clear_map_data_ffi")()

/proc/_bapidmm_parse_map_blocking(dmm_file)
	return call_ext(BAPI_DMM_READER, "byond:_bapidmm_parse_map_blocking_ffi")(dmm_file)

/proc/_bapidmm_test_connection()
	return call_ext(BAPI_DMM_READER, "byond:_bapidmm_test_connection_ffi")()

