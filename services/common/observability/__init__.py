import os
from typing import Optional

from fastapi import FastAPI
from opentelemetry import trace
from opentelemetry.exporter.otlp.proto.grpc.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor, SpanExporter, SpanProcessor
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.instrumentation.requests import RequestsInstrumentor
from opentelemetry.instrumentation.sqlalchemy import SQLAlchemyInstrumentor
# Redis instrumentation imported lazily to avoid hard dependency when redis is absent
from prometheus_fastapi_instrumentator import Instrumentator


SENSITIVE_ATTR_KEYS = {"user", "user_id", "email", "subject", "device_id"}


class SensitiveAttributeFilterSpanProcessor(SpanProcessor):
	"""
	Removes sensitive attributes from spans before export. This ensures we don't
	accidentally propagate PII into traces.
	"""

	def __init__(self, delegate: SpanProcessor) -> None:
		self._delegate = delegate

	def on_start(self, span, parent_context) -> None:  # type: ignore[override]
		self._delegate.on_start(span, parent_context)

	def on_end(self, span) -> None:  # type: ignore[override]
		# Drop known sensitive attributes if present
		for key in list(span.attributes.keys()):
			if key in SENSITIVE_ATTR_KEYS:
				try:
					del span.attributes[key]
				except Exception:
					# Best-effort; do not raise in hot path
					pass
		self._delegate.on_end(span)

	def shutdown(self) -> None:  # type: ignore[override]
		self._delegate.shutdown()

	def force_flush(self, timeout_millis: Optional[int] = None) -> bool:  # type: ignore[override]
		return self._delegate.force_flush(timeout_millis)


def init_observability(app: FastAPI, service_name: str) -> None:
	"""
	Initialize OpenTelemetry tracing and Prometheus metrics for a FastAPI service.
	- Traces: OTLP gRPC exporter (defaults to jaeger:4317), PII scrubbed
	- Metrics: /metrics via PrometheusFastApiInstrumentator with grouped paths
	"""
	# Tracing
	resource = Resource.create(
		{
			"service.name": service_name,
			"service.namespace": "vpn-mvp",
			"service.version": os.getenv("SERVICE_VERSION", "0.1.0"),
			"deployment.environment": os.getenv("DEPLOY_ENV", "dev"),
		}
	)
	tracer_provider = TracerProvider(resource=resource)
	trace.set_tracer_provider(tracer_provider)

	otlp_endpoint = os.getenv("OTEL_EXPORTER_OTLP_GRPC_ENDPOINT", "http://jaeger:4317")
	span_exporter: SpanExporter = OTLPSpanExporter(endpoint=otlp_endpoint, insecure=True)
	batch_processor = BatchSpanProcessor(span_exporter)
	# Wrap with sensitive attribute filter
	tracer_provider.add_span_processor(SensitiveAttributeFilterSpanProcessor(batch_processor))

	# Instrumentations
	FastAPIInstrumentor.instrument_app(app, tracer_provider=tracer_provider)
	RequestsInstrumentor().instrument()
	try:
		SQLAlchemyInstrumentor().instrument()
	except Exception:
		# Service may not use SQLAlchemy; ignore
		pass
	try:
		# Import lazily to prevent hard dependency in services that don't use redis
		from opentelemetry.instrumentation.redis import RedisInstrumentor  # type: ignore

		RedisInstrumentor().instrument()
	except Exception:
		# Service may not use Redis; ignore
		pass

	# Metrics (basic instrumentation/expose; avoid per-version API differences)
	Instrumentator().instrument(app).expose(app, include_in_schema=False)


