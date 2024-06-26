# world.gd
extends Node3D

const chunk_size = 64
const chunk_amount = 16

var noise
var chunks = {}
var unready_chunks = {}
var thread

func _ready():
	randomize()
	#noise = OpenSimplexNoise.new()
	noise = FastNoiseLite.new()
	noise.seed = randi()
	noise.octaves = 6
	noise.period = 80

	thread = Thread.new()

func add_chunk(x,z):
	var key = str(x) + "," + str(z)
	if chunks.has(key) or unready_chunks.has(key):
		return

	if not thread.is_active():
		thread.start(self, "load_chunk", [thread, x, z])
		unready_chunks[key] = 1

func load_chunk(arr):
	var thread = arr[0]
	var x = arr[1]
	var z = arr[2]

	var chunk = Chunk.new(noise, x*chunk_size, z*chunk_size, chunk_size)
	chunk.translation = Vector3(x*chunk_size, 0, z*chunk_size)

	call_deferred("load_done", chunk, thread)

func load_done(chunk, thread):
	add_child(chunk)
	var key = str(chunk.x / chunk_size) + "," + str(chunk.z/ chunk_size)
	chunks[key] = chunk
	unready_chunks.erase(key)
	thread.wait_to_finish()

func get_chunk(x, z):
	var key = str(x) + "," + str(z)
	if chunks.has(key):
		return chunks.get(key)

	return null

func _process(delta):
	update_chunks()
	clean_up_chunks()
	reset_chunks()

func update_chunks():
	var player_translateion = $Player.translation # update to retrive submarine prosition
	var p_x = int(player_translateion.x) / chunk_size
	var p_z = int(player_translateion.z) / chunk_size

	for x in range(p_x - chunk_amount * 0.5, p_x + chunk_amount * 0.5):
		for z in range(p_z - chunk_amount * 0.5, p_z + chunk_amount * 0.5):
			add_chunk(x, z)

func clean_up_chunks():
	pass

func reset_chunks():
	pass
